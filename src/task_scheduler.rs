use std::env;
use windows::core::{Interface, BSTR};
use windows::Win32::Foundation::{VARIANT_FALSE, VARIANT_TRUE};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::System::TaskScheduler::{
    IAction, IActionCollection, IBootTrigger, IDailyTrigger, IEventTrigger, IExecAction,
    IIdleSettings, IIdleTrigger, ILogonTrigger, IMaintenanceSettings, IMonthlyDOWTrigger,
    IMonthlyTrigger, IRegistrationInfo, IRegistrationTrigger, ISessionStateChangeTrigger,
    ITaskDefinition, ITaskFolder, ITaskService, ITaskSettings, ITaskSettings2, ITaskSettings3,
    ITimeTrigger, ITriggerCollection, IWeeklyTrigger, TaskScheduler, TASK_ACTION_EXEC,
    TASK_COMPATIBILITY_V2_4, TASK_CONSOLE_CONNECT, TASK_CONSOLE_DISCONNECT, TASK_CREATE_OR_UPDATE,
    TASK_INSTANCES_IGNORE_NEW, TASK_LOGON_INTERACTIVE_TOKEN, TASK_REMOTE_CONNECT,
    TASK_REMOTE_DISCONNECT, TASK_RUNLEVEL_HIGHEST, TASK_SESSION_LOCK, TASK_SESSION_UNLOCK,
    TASK_TRIGGER_BOOT, TASK_TRIGGER_DAILY, TASK_TRIGGER_EVENT, TASK_TRIGGER_IDLE,
    TASK_TRIGGER_LOGON, TASK_TRIGGER_MONTHLY, TASK_TRIGGER_MONTHLYDOW, TASK_TRIGGER_REGISTRATION,
    TASK_TRIGGER_SESSION_STATE_CHANGE, TASK_TRIGGER_TIME, TASK_TRIGGER_WEEKLY,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

// 创建Windows任务计划:
// https://docs.microsoft.com/zh-cn/windows/win32/taskschd/task-scheduler-start-page
// https://learn.microsoft.com/zh-cn/windows/win32/api/_taskschd
// 解锁、启动、登录等事件触发任务计划 taskschd.msc
pub fn create_schedule() -> anyhow::Result<(), Box<dyn std::error::Error>> {
    // 获取当前执行程序的路径
    let exe_path = env::current_exe()?;
    let exe_path_str = exe_path.to_str().ok_or_else(|| std::io::Error::last_os_error())?;

    // 将所有 Windows API 操作放进一个闭包中
    // 闭包的返回值强制指定为 windows::core::Result<()>
    let setup_set = || -> windows::core::Result<()> {
        unsafe {
            // 1. 初始化 COM
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            // 初始化Windows运行时
            // 可能返回：
            // windows::Win32::Foundation::S_OK：成功初始化
            // windows::Win32::Foundation::S_FALSE：该线程已经初始化
            // RPC_E_CHANGED_MODE：并发模型冲突
            RoInitialize(RO_INIT_MULTITHREADED)?;
            // RoInitialize(RO_INIT_MULTITHREADED).map_err(|e| windows::core::Error::new(e.code(), e.message()))?;
            // RoInitialize(RO_INIT_SINGLETHREADED)?;

            // 2. 创建任务服务实例并连接
            let task_service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_ALL)?;
            task_service.Connect(
                &VARIANT::default(), // Server
                &VARIANT::default(), // User
                &VARIANT::default(), // Domain
                &VARIANT::default(), // Password
            )?;

            // 3. 获取根文件夹并创建新任务定义
            // 默认位置 "\"
            let task_folder: ITaskFolder = task_service.GetFolder(&BSTR::from("\\"))?;
            let task_definition: ITaskDefinition = task_service.NewTask(0)?;

            // ==========================================
            // 1. 常规 (General)
            // ==========================================

            // 注册信息：作者与描述
            let registration_info: IRegistrationInfo = task_definition.RegistrationInfo()?;
            // 名称 (Name): 注册时在 RegisterTaskDefinition 中指定
            // 位置 (Location): 由 task_folder 决定
            // 作者 (Author)
            registration_info.SetAuthor(&BSTR::from("Bajins"))?;
            // 描述 (Description)
            registration_info.SetDescription(&BSTR::from(
                "动态设置桌面壁纸 — https://github.com/bajins/desktop-wallpaper-rust",
            ))?;

            // 安全选项 (Security Options)
            let principal = task_definition.Principal()?;

            // 运行账户 (User/Group): 默认为当前用户
            // principal.SetUserId(&BSTR::from("SYSTEM"))?;

            // 登录选项 (SetLogonType)
            // 可选值：
            // TASK_LOGON_NONE: 未指定
            // TASK_LOGON_PASSWORD: 使用密码（交互或后台）
            // TASK_LOGON_S4U: 不管用户是否登录都要运行（不存储密码 S4U）
            // TASK_LOGON_INTERACTIVE_TOKEN: 只在用户登录时运行
            // TASK_LOGON_GROUP: 组账户
            // TASK_LOGON_SERVICE_ACCOUNT: 服务账户（如 LocalService, NetworkService）
            // TASK_LOGON_INTERACTIVE_TOKEN_OR_PASSWORD: 登录或有密码时
            principal.SetLogonType(TASK_LOGON_INTERACTIVE_TOKEN)?;

            // 使用最高权限运行 (Run with highest privileges)
            // 可选值：
            // TASK_RUNLEVEL_LUA: 标准用户权限
            // TASK_RUNLEVEL_HIGHEST: 管理员/最高权限
            principal.SetRunLevel(TASK_RUNLEVEL_HIGHEST)?;

            // 隐藏 (Hidden)
            task_definition.Settings()?.SetHidden(VARIANT_FALSE)?;

            // 配置 (Configure for) / 兼容性设置 (SetCompatibility)
            // 可选值：
            // TASK_COMPATIBILITY_V1: Windows Server 2003, Windows XP, or Windows 2000
            // TASK_COMPATIBILITY_V2: Windows Vista, Windows Server 2008
            // TASK_COMPATIBILITY_V2_1: Windows 7, Windows Server 2008 R2
            // TASK_COMPATIBILITY_V2_2: Windows 8
            // TASK_COMPATIBILITY_V2_3: Windows 8.1
            // TASK_COMPATIBILITY_V2_4: Windows 10
            task_definition
                .Settings()?
                .SetCompatibility(TASK_COMPATIBILITY_V2_4)?;

            // ==========================================
            // 2. 触发器 (Triggers)
            // ==========================================
            // ISO 8601 duration format (e.g., "PT30M" for 30 minutes) P[nY][nM][nD][T[nH][nM][nS]]
            // PTnM：n 分钟（如 PT15M）。PTnH：n 小时（如 PT1H）。PnD：n 天（如 P1D）。PT0S：立即。

            let triggers: ITriggerCollection = task_definition.Triggers()?;

            // --- 2.1 按预定计划 - 一次 ---
            let t_once = triggers.Create(TASK_TRIGGER_TIME)?;
            let i_once_trigger: ITimeTrigger = t_once.cast()?;
            i_once_trigger.SetId(&BSTR::from("Trigger_Once"))?;
            // 开始时间
            i_once_trigger.SetStartBoundary(&BSTR::from("2024-01-01T12:00:00"))?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间 (RandomDelay)
            i_once_trigger.SetRandomDelay(&BSTR::from("PT1M"))?;
            let rep_once = i_once_trigger.Repetition()?;
            // 重复任务间隔
            rep_once.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_once.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_once.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_once_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 过期时间 (Expire)
            i_once_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
            // 启用
            i_once_trigger.SetEnabled(VARIANT_FALSE)?;

            // --- 2.2 按预定计划 - 每天 ---
            let t_daily = triggers.Create(TASK_TRIGGER_DAILY)?;
            let i_daily_trigger: IDailyTrigger = t_daily.cast()?;
            i_daily_trigger.SetStartBoundary(&BSTR::from("2024-01-01T08:00:00"))?;
            // 每隔多少天发生一次
            i_daily_trigger.SetDaysInterval(1)?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间 (RandomDelay)
            i_daily_trigger.SetRandomDelay(&BSTR::from("PT1M"))?;
            let rep_daily = i_daily_trigger.Repetition()?;
            // 重复任务间隔
            rep_daily.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_daily.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_daily.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_daily_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 过期时间 (Expire)
            i_daily_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
            // 启用
            i_daily_trigger.SetEnabled(VARIANT_FALSE)?;

            // --- 2.3 按预定计划 - 每周 ---
            let t_weekly = triggers.Create(TASK_TRIGGER_WEEKLY)?;
            let i_weekly_trigger: IWeeklyTrigger = t_weekly.cast()?;
            i_weekly_trigger.SetStartBoundary(&BSTR::from("2024-01-01T08:00:00"))?;
            // 每隔多少周
            i_weekly_trigger.SetWeeksInterval(1)?;
            // 星期几 (Bitmask: Sunday=1, Monday=2, Tuesday=4, Wed=8, Thu=16, Fri=32, Sat=64)
            i_weekly_trigger.SetDaysOfWeek(2)?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间 (RandomDelay)
            i_weekly_trigger.SetRandomDelay(&BSTR::from("PT1M"))?;
            let rep_weekly = i_weekly_trigger.Repetition()?;
            // 重复任务间隔
            rep_weekly.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_weekly.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_weekly.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_weekly_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 过期时间 (Expire)
            i_weekly_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
            // 启用
            i_weekly_trigger.SetEnabled(VARIANT_FALSE)?;

            // --- 2.4 按预定计划 - 每月 ---
            let t_monthly = triggers.Create(TASK_TRIGGER_MONTHLY)?;
            let i_monthly_trigger: IMonthlyTrigger = t_monthly.cast()?;
            i_monthly_trigger.SetStartBoundary(&BSTR::from("2024-01-01T08:00:00"))?;
            // 月 (Bitmask: Jan=1, Feb=2, Mar=4, Apr=8, May=16, Jun=32, Jul=64, Aug=128, Sep=256, Oct=512, Nov=1024, Dec=2048)
            i_monthly_trigger.SetMonthsOfYear(0x0FFF)?;
            // 天
            i_monthly_trigger.SetDaysOfMonth(1)?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间 (RandomDelay)
            i_monthly_trigger.SetRandomDelay(&BSTR::from("PT1M"))?;
            let rep_monthly = i_monthly_trigger.Repetition()?;
            // 重复任务间隔
            rep_monthly.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_monthly.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_monthly.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_monthly_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 过期时间 (Expire)
            i_monthly_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
            // 启用
            i_monthly_trigger.SetEnabled(VARIANT_FALSE)?;

            // --- 2.5 按预定计划 - 在第几个星期几 ---
            let t_mdow = triggers.Create(TASK_TRIGGER_MONTHLYDOW)?;
            let i_mdow_trigger: IMonthlyDOWTrigger = t_mdow.cast()?;
            i_mdow_trigger.SetDaysOfWeek(2)?;
            // 第几个星期 (Bitmask: First=1, Second=2, Third=4, Fourth=8, Last=16)
            i_mdow_trigger.SetWeeksOfMonth(1)?;
            i_mdow_trigger.SetMonthsOfYear(0x0FFF)?;
            // 开始时间
            i_mdow_trigger.SetStartBoundary(&BSTR::from("2024-01-01T08:00:00"))?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间 (RandomDelay)
            i_mdow_trigger.SetRandomDelay(&BSTR::from("PT1M"))?;
            let rep_mdow = i_mdow_trigger.Repetition()?;
            // 重复任务间隔
            rep_mdow.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_mdow.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_mdow.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_mdow_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 过期时间 (Expire)
            i_mdow_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
            // 启用
            i_mdow_trigger.SetEnabled(VARIANT_FALSE)?;

            // --- 2.6 登录时 ---
            let t_logon = triggers.Create(TASK_TRIGGER_LOGON)?;
            let i_logon_trigger: ILogonTrigger = t_logon.cast()?;
            // 特定用户 (如果不设置则为“所有用户”)
            // i_logon_trigger.SetUserId(&BSTR::from("DOMAIN\\User"))?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_logon_trigger.SetDelay(&BSTR::from("PT2M"))?;
            /*let rep_logon = t_logon.Repetition()?;
            // 重复任务间隔
            rep_logon.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_logon.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_logon.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            t_logon.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            i_logon_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            t_logon.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;*/
            // 启用
            t_logon.SetEnabled(VARIANT_TRUE)?;

            // --- 2.7 启动时 ---
            let t_boot = triggers.Create(TASK_TRIGGER_BOOT)?;
            let i_boot_trigger: IBootTrigger = t_boot.cast()?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_boot_trigger.SetDelay(&BSTR::from("PT1M"))?;
            /*let rep_boot = i_boot_trigger.Repetition()?;
            // 重复任务间隔
            rep_boot.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_boot.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_boot.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_boot_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            i_boot_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            i_boot_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;*/
            // 启用
            i_boot_trigger.SetEnabled(VARIANT_TRUE)?;

            // --- 2.8 空闲状态 ---
            let t_idle = triggers.Create(TASK_TRIGGER_IDLE)?;
            let i_idle_trigger: IIdleTrigger = t_idle.cast()?;
            // --- 高级设置 (Advanced Settings) ---
            let rep_idle = i_idle_trigger.Repetition()?;
            // 重复任务间隔
            rep_idle.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_idle.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_idle.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_idle_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            i_idle_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            i_idle_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
            // 启用
            i_idle_trigger.SetEnabled(VARIANT_FALSE)?;

            // --- 2.9 发生事件时 ---
            let t_event = triggers.Create(TASK_TRIGGER_EVENT)?;
            let i_event_trigger: IEventTrigger = t_event.cast()?;
            // 基本设置 (通过 XML 定义筛选器)
            i_event_trigger.SetSubscription(&BSTR::from(r"<QueryList><Query Id='0'><Select Path='System'>*[System[(EventID=100)]]</Select></Query></QueryList>"))?;
            // 定义事件查询。触发器将启动任务，当收到事件时。
            /*i_event_trigger.SetSubscription(&BSTR::from(r"<QueryList>
                <Query Id='0'>
                    <Select Path='System'>
                        *[System[Provider[@Name='Microsoft-Windows-Power-Troubleshooter'] and EventID=1]]
                    </Select>
                </Query>
                <Query Id='1'>
                    <Select Path='System'>
                        *[System/Level=2]
                    </Select>
                </Query>
            </QueryList>"))?;*/
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_event_trigger.SetDelay(&BSTR::from("PT1M"))?;
            let rep_event = i_event_trigger.Repetition()?;
            // 重复任务间隔
            rep_event.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_event.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_event.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_event_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            i_event_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            i_event_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
            // 启用
            i_event_trigger.SetEnabled(VARIANT_FALSE)?;

            // --- 2.10 创建/修改任务时 ---
            let t_reg = triggers.Create(TASK_TRIGGER_REGISTRATION)?;
            let i_reg_trigger: IRegistrationTrigger = t_reg.cast()?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_reg_trigger.SetDelay(&BSTR::from("PT1M"))?;
            let rep_reg = i_reg_trigger.Repetition()?;
            // 重复任务间隔
            rep_reg.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_reg.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_reg.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_reg_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            i_reg_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            i_reg_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
            // 启用
            i_reg_trigger.SetEnabled(VARIANT_FALSE)?;

            // --- 2.11 会话更改 (连接/断开/锁定/解锁) ---
            // 状态变更类型 (SetStateChange)

            // TASK_CONSOLE_CONNECT: 连接到本地会话
            let t_tssc_cc = triggers.Create(TASK_TRIGGER_SESSION_STATE_CHANGE)?;
            let i_tssc_cc_trigger: ISessionStateChangeTrigger = t_tssc_cc.cast()?;
            i_tssc_cc_trigger.SetStateChange(TASK_CONSOLE_CONNECT)?;
            // 特定用户
            // i_tssc_cc_trigger.SetUserId(&BSTR::from("bajins"))?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_tssc_cc_trigger.SetDelay(&BSTR::from("PT10S"))?;
            /*let rep_tssc_cc = i_tssc_cc_trigger.Repetition()?;
            // 重复任务间隔
            rep_tssc_cc.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_tssc_cc.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_tssc_cc.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_tssc_cc_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            i_tssc_cc_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            i_tssc_cc_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;*/
            // 启用
            i_tssc_cc_trigger.SetEnabled(VARIANT_TRUE)?;

            // TASK_CONSOLE_DISCONNECT: 从本地会话断开
            let t_tssc_cd = triggers.Create(TASK_TRIGGER_SESSION_STATE_CHANGE)?;
            let i_tssc_cd_trigger: ISessionStateChangeTrigger = t_tssc_cd.cast()?;
            i_tssc_cd_trigger.SetStateChange(TASK_CONSOLE_DISCONNECT)?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_tssc_cd_trigger.SetDelay(&BSTR::from("PT10S"))?;
            /*let rep_tssc_cd = i_tssc_cd_trigger.Repetition()?;
            // 重复任务间隔
            rep_tssc_cd.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_tssc_cd.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_tssc_cd.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_tssc_cd_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            i_tssc_cd_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            i_tssc_cd_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;*/
            // 启用
            i_tssc_cd_trigger.SetEnabled(VARIANT_FALSE)?;

            // TASK_REMOTE_CONNECT: 连接到远程会话
            let t_tssc_rc = triggers.Create(TASK_TRIGGER_SESSION_STATE_CHANGE)?;
            let i_tssc_rc_trigger: ISessionStateChangeTrigger = t_tssc_rc.cast()?;
            i_tssc_rc_trigger.SetStateChange(TASK_REMOTE_CONNECT)?;
            // 特定用户
            // i_tssc_rc_trigger.SetUserId(&BSTR::from("bajins"))?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_tssc_rc_trigger.SetDelay(&BSTR::from("PT10S"))?;
            /*let rep_tssc_rc = i_tssc_rc_trigger.Repetition()?;
            // 重复任务间隔
            rep_tssc_rc.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_tssc_rc.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_tssc_rc.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_tssc_rc_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            i_tssc_rc_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            i_tssc_rc_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;*/
            // 启用
            i_tssc_rc_trigger.SetEnabled(VARIANT_FALSE)?;

            // TASK_REMOTE_DISCONNECT: 从远程会话断开
            let t_tssc_rd = triggers.Create(TASK_TRIGGER_SESSION_STATE_CHANGE)?;
            let i_tssc_rd_trigger: ISessionStateChangeTrigger = t_tssc_rd.cast()?;
            i_tssc_rd_trigger.SetStateChange(TASK_REMOTE_DISCONNECT)?;
            // 特定用户
            // i_tssc_rd_trigger.SetUserId(&BSTR::from("bajins"))?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_tssc_rd_trigger.SetDelay(&BSTR::from("PT10S"))?;
            /*let rep_tssc_rd = i_tssc_rd_trigger.Repetition()?;
            // 重复任务间隔
            rep_tssc_rd.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_tssc_rd.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_tssc_rd.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_tssc_rd_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            i_tssc_rd_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            i_tssc_rd_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;*/
            // 启用
            i_tssc_rd_trigger.SetEnabled(VARIANT_FALSE)?;

            // TASK_SESSION_LOCK: 工作站锁定
            let t_tssc_sl = triggers.Create(TASK_TRIGGER_SESSION_STATE_CHANGE)?;
            let i_tssc_sl_trigger: ISessionStateChangeTrigger = t_tssc_sl.cast()?;
            i_tssc_sl_trigger.SetStateChange(TASK_SESSION_LOCK)?;
            // 特定用户
            // i_tssc_sl_trigger.SetUserId(&BSTR::from("bajins"))?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_tssc_sl_trigger.SetDelay(&BSTR::from("PT10S"))?;
            /*let rep_tssc_sl = i_tssc_sl_trigger.Repetition()?;
            // 重复任务间隔
            rep_tssc_sl.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_tssc_sl.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_tssc_sl.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_tssc_sl_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            i_tssc_sl_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            i_tssc_sl_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;*/
            // 启用
            i_tssc_sl_trigger.SetEnabled(VARIANT_FALSE)?;

            // TASK_SESSION_UNLOCK: 工作站解锁
            let t_tssc_su = triggers.Create(TASK_TRIGGER_SESSION_STATE_CHANGE)?;
            let i_tssc_su_trigger: ISessionStateChangeTrigger = t_tssc_su.cast()?;
            i_tssc_su_trigger.SetStateChange(TASK_SESSION_UNLOCK)?;
            // 特定用户
            // i_tssc_su_trigger.SetUserId(&BSTR::from("bajins"))?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_tssc_su_trigger.SetDelay(&BSTR::from("PT10S"))?;
            /*let rep_tssc_su = i_tssc_su_trigger.Repetition()?;
            // 重复任务间隔
            rep_tssc_su.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_tssc_su.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_tssc_su.SetStopAtDurationEnd(VARIANT_TRUE)?;
            // 运行时间超过此值则停止执行
            i_tssc_su_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            i_tssc_su_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            i_tssc_su_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;*/
            // 启用
            i_tssc_su_trigger.SetEnabled(VARIANT_TRUE)?;

            // ==========================================
            // 3. 操作 (Actions)
            // ==========================================
            let actions: IActionCollection = task_definition.Actions()?;
            // 操作类型: 启动程序
            let action: IAction = actions.Create(TASK_ACTION_EXEC)?;
            let exec_action: IExecAction = action.cast::<IExecAction>()?;

            //i_exec_action.SetId(&BSTR::from("set_bing_wallpaper"))
            // 程序/脚本 (Program/script)
            exec_action.SetPath(&BSTR::from(exe_path_str))?;
            // 添加参数 (Add arguments)
            exec_action.SetArguments(&BSTR::from(""))?;
            // 起始于 (Start in)
            if let Some(parent) = exe_path.parent() {
                if let Some(dir_str) = parent.to_str() {
                    exec_action.SetWorkingDirectory(&BSTR::from(dir_str))?;
                }
            }

            // ==========================================
            // 4. 条件 (Conditions)
            // ==========================================
            let settings: ITaskSettings = task_definition.Settings()?;

            // 空闲 (Idle)
            let idle_settings: IIdleSettings = settings.IdleSettings()?;
            // 仅当计算机空闲时间超过下列值时才启动此任务
            /*idle_settings.SetIdleDuration(&BSTR::from("PT10M"))?;
            // 等待空闲时间
            idle_settings.SetWaitTimeout(&BSTR::from("PT1H"))?;*/
            // 如果计算机不再空闲，则停止
            idle_settings.SetStopOnIdleEnd(VARIANT_FALSE)?;
            // 如果空闲状态继续，则重新启动
            idle_settings.SetRestartOnIdle(VARIANT_FALSE)?;

            // 电源 (Power)
            // 只有在计算机使用交流电源时才启动此任务
            settings.SetDisallowStartIfOnBatteries(VARIANT_FALSE)?;
            // 如果计算机改用电池电源，则停止
            settings.SetStopIfGoingOnBatteries(VARIANT_FALSE)?;
            // 唤醒计算机运行此任务
            settings.SetWakeToRun(VARIANT_TRUE)?;

            // 网络 (Network)
            // 仅当特定网络连接可用时才启动
            settings.SetRunOnlyIfNetworkAvailable(VARIANT_TRUE)?;
            // 使用 ITaskSettings3 完善特定网络详情
            //let settings3: ITaskSettings3 = settings.cast()?;
            //let net_settings: INetworkSettings = settings3.NetworkSettings()?;
            // 如果需要特定网络，设置网络名称或 ID
            // net_settings.SetName(&BSTR::from("My WiFi Name"))?;
            // net_settings.SetId(&BSTR::from("{GUID}"))?;

            // ==========================================
            // 5. 设置 (Settings)
            // ==========================================

            // 允许按需运行
            settings.SetAllowDemandStart(VARIANT_TRUE)?;
            // 如果过了计划开始时间，立即启动任务
            settings.SetStartWhenAvailable(VARIANT_TRUE)?;
            // 如果任务失败，按以下频率重新启动
            settings.SetRestartInterval(&BSTR::from("PT1M"))?;
            // 尝试重新启动最多次数
            settings.SetRestartCount(3)?;
            // 如果任务运行时间超过以下时间，停止任务
            settings.SetExecutionTimeLimit(&BSTR::from("PT5M"))?;
            // 如果请求后任务还在运行，强行将其停止
            settings.SetAllowHardTerminate(VARIANT_TRUE)?;
            // 如果任务没有计划再次运行，则在此之后删除该任务
            settings.SetDeleteExpiredTaskAfter(&BSTR::from("PT0S"))?;
            // 如果此任务已经运行，以下规则适用 (Multiple Instances)
            // 可选值：
            // TASK_INSTANCES_PARALLEL: 并行运行新实例
            // TASK_INSTANCES_QUEUE: 对新实例排队
            // TASK_INSTANCES_IGNORE_NEW: 请勿启动新实例
            // TASK_INSTANCES_STOP_EXISTING: 停止现有实例
            settings.SetMultipleInstances(TASK_INSTANCES_IGNORE_NEW)?;

            // Windows 7+
            let settings2: ITaskSettings2 = settings.cast()?;
            // 控制任务是否可以在 RemoteApp (RAIL) 会话中启动
            settings2.SetDisallowStartOnRemoteAppSession(VARIANT_FALSE)?;
            // 是否使用统一调度引擎（Unified Scheduling Engine，性能更好、更现代的引擎）
            // 统一调度引擎是 Windows 7 及更高版本引入的一种机制，旨在让“任务计划程序”和“Windows 服务”的行为更加一致，提高性能
            settings2.SetUseUnifiedSchedulingEngine(VARIANT_TRUE)?;

            // Windows 8+
            if let Ok(settings3) = settings.cast::<ITaskSettings3>() {
                // 易失性任务。如果设为 True，任务在系统重启后会自动禁用（常用于集群环境或临时任务）。
                // settings3.SetVolatile(VARIANT_TRUE)?;

                // 自动维护设置。允许任务在 Windows 系统系统闲置（通常是凌晨）且插着电源时运行，可以设置周期和死限。
                let maint_settings: IMaintenanceSettings = settings3.CreateMaintenanceSettings()?;
                // 设置周期（例如：每 1 天运行一次）
                maint_settings.SetPeriod(&BSTR::from("P1D"))?;
                // 设置截止限：必须严格大于 Period（例如：如果 2 天都没跑，则强制运行）
                maint_settings.SetDeadline(&BSTR::from("P2D"))?;
                // 是否要求独占（系统闲置不运行其他任务）
                //maint_settings.SetExclusive(VARIANT_FALSE)?;
            }

            // ==========================================
            // 6. 注册任务 (Register Task)
            // ==========================================
            // 把所有字节清零（相当于 C 的 memset(&var, 0, sizeof(VARIANT))）
            // let mut empty_var = VARIANT::default();
            // 对 C 语言 匿名联合体（anonymous union） 的模拟
            // (*empty_var.Anonymous.Anonymous).vt = VT_EMPTY;

            let registered_task = task_folder
                .RegisterTaskDefinition(
                    &BSTR::from("SetBingWallpaper"),
                    &task_definition,
                    TASK_CREATE_OR_UPDATE.0,
                    &VARIANT::default(),          // UserID
                    &VARIANT::default(),          // &empty_var // Password
                    TASK_LOGON_INTERACTIVE_TOKEN, // LogonType
                    &VARIANT::default(),          // sddl
                )
                // .map(|_| ()) // 把 IRegisteredTask 转成 ()
                .map_err(|e| e)?;
            /*.map_err(|e| {
                windows::core::Error::new(
                    windows::Win32::Foundation::S_FALSE,
                    format!(
                        "RegisterTaskDefinition failed: {} (HRESULT: 0x{:08X})",
                        e.message(),
                        e.code().0
                    ),
                )
            })?;*/

            registered_task.SetEnabled(VARIANT_TRUE)?;

            // println!("任务计划已成功注册！");

            Ok(())
        }
    };
    // 执行闭包，如果发生错误，通过 .map_err 将其转为 String
    // 这样 String 就能被 ? 成功转为 Box<dyn Error> 抛出去了！
    setup_set().map_err(|e| e.to_string())?;

    unsafe {
        // 关闭Windows运行时
        RoUninitialize();
        // 释放COM资源
        CoUninitialize();
    }

    Ok(())
}

/// 注册开机启动的函数
pub fn add_to_startup(
    app_name: &str,
    app_path: &str,
) -> anyhow::Result<(), Box<dyn std::error::Error>> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
    let (key, _disp) = hklm.create_subkey(&path)?;

    key.set_value(app_name, &app_path)?;
    Ok(())
}
