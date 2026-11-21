use crate::utils;
use std::env;
use std::time::Duration;
use windows::core::{BSTR, HRESULT, Interface, PCWSTR};
use windows::Win32::Foundation::{ERROR_SUCCESS, VARIANT_FALSE, VARIANT_TRUE, WIN32_ERROR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::System::TaskScheduler::{
    IAction, IActionCollection, IBootTrigger, IDailyTrigger, IEventTrigger, IExecAction,
    IIdleSettings, IIdleTrigger, ILogonTrigger, /* IMaintenanceSettings, */ IMonthlyDOWTrigger,
    IMonthlyTrigger, IRegistrationInfo, IRegistrationTrigger, ISessionStateChangeTrigger,
    ITaskDefinition, ITaskFolder, ITaskService, ITaskSettings, ITaskSettings2, /* ITaskSettings3, */
    ITimeTrigger, ITriggerCollection, IWeeklyTrigger, TaskScheduler, TASK_ACTION_EXEC,
    TASK_COMPATIBILITY_V2_4, TASK_CONSOLE_CONNECT, TASK_CONSOLE_DISCONNECT, TASK_CREATE_OR_UPDATE
    , TASK_INSTANCES_IGNORE_NEW, TASK_LOGON_INTERACTIVE_TOKEN
    , TASK_REMOTE_CONNECT, TASK_REMOTE_DISCONNECT, TASK_RUNLEVEL_HIGHEST,
    TASK_SESSION_LOCK, TASK_SESSION_UNLOCK, TASK_TRIGGER_BOOT, TASK_TRIGGER_DAILY,
    TASK_TRIGGER_EVENT, TASK_TRIGGER_IDLE, TASK_TRIGGER_LOGON, TASK_TRIGGER_MONTHLY,
    TASK_TRIGGER_MONTHLYDOW, TASK_TRIGGER_REGISTRATION, TASK_TRIGGER_SESSION_STATE_CHANGE,
    TASK_TRIGGER_TIME, TASK_TRIGGER_WEEKLY,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_WRITE,
    REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

/// 事件触发器订阅 XML（QueryList 查询集）
///
/// 停用策略：全量/高频事件查询（System 全量错误、4624 全量登录、Winlogon 7001/7002、
/// 4800/4801 锁定解锁）均已注释停用——事件触发器每命中一次即启动一个新实例，
/// 高频命中叠加不限制并行的多实例策略会形成进程风暴导致系统卡顿；
/// 登录/解锁场景已由任务级"登录时""会话更改"触发器覆盖。
/// 恢复任一查询前，务必确认任务多实例策略为"请勿启动新实例"，
/// 且程序内已具备单实例锁与运行时长自限两道兜底。
const EVENT_SUBSCRIPTION_XML: &str = r#"<QueryList>
    <!-- 查询 0：电源故障排除事件（系统从睡眠/休眠恢复、电源状态变化等）。
         保留启用：每次唤醒仅触发一次，用于唤醒后刷新壁纸，频率低且合理 -->
    <Query Id='0' Path='System'>
        <Select Path='System'>
            *[System[Provider[@Name='Microsoft-Windows-Power-Troubleshooter'] and EventID=1]]
        </Select>
    </Query>

    <!-- 查询 1（已停用）：System 日志全量 Level=2（错误）事件。
         停用原因：任何系统组件写错误日志都会触发一次任务启动，频率完全不可控，
         还存在"任务失败→记录错误事件→再次触发"的自激循环风险，是进程风暴的主要源头之一
    <Query Id='1' Path='System'>
        <Select Path='System'>
            *[System/Level=2]
        </Select>
    </Query> -->

    <!-- 查询 2（已停用）：4624 全量登录事件（原语义：交互式/解锁等登录，需开启"审核登录事件"）。
         停用原因：Windows 服务登录（LogonType=5）、网络登录等每天产生数百条 4624，
         未按 LogonType 过滤时每条都会触发一次任务启动；登录场景已由"登录时"触发器覆盖
    <Query Id='2' Path='Security'>
        <Select Path='Security'>
            *[System[Provider[@Name='Microsoft-Windows-Security-Auditing'] and EventID=4624]]
        </Select>
    </Query> -->

    <!-- 查询 3（历史参考，原本就未启用）：仅匹配登录类型 2：Interactive（本地键盘/屏幕登录） -->
    <!-- <Query Id='3' Path='Security'>
        <Select Path='Security'>
            *[System[Provider[@Name='Microsoft-Windows-Security-Auditing'] and EventID=4624]]
            and
            *[EventData[Data[@Name='LogonType']='2']]
        </Select>
    </Query> -->
    <!-- <Query Id="0" Path="Security">
        <Select Path="Security">
            *[System[(EventID=4624)]]
            and
            *[EventData[Data[@Name='LogonType']='2']]
        </Select>
    </Query> -->

    <!-- 查询 4（历史参考，原本就未启用）：仅匹配登录类型 7：Unlock（通过 4624 事件识别的解锁） -->
    <!-- <Query Id='4' Path='Security'>
        <Select Path='Security'>
            *[System[Provider[@Name='Microsoft-Windows-Security-Auditing'] and EventID=4624]]
            and
            *[EventData[Data[@Name='LogonType']='7']]
        </Select>
    </Query> -->

    <!-- 查询 5（已停用）：工作站解锁 4801（需启用"审核其他登录/注销事件"）。
         停用原因：与"会话更改-工作站解锁"触发器语义重复，一次解锁会双重触发
    <Query Id='5' Path='Security'>
        <Select Path='Security'>
            *[System[Provider[@Name='Microsoft-Windows-Security-Auditing'] and EventID=4801]]
        </Select>
    </Query> -->
    <!-- <Query Id="5" Path="Security">
        <Select Path="Security">
            *[System[(EventID=4801)]]
        </Select>
    </Query> -->

    <!-- 查询 6（已停用）：工作站锁定 4800（需额外启用审计）。
         停用原因：锁定时用户看不到桌面，无需刷新壁纸，且与"会话更改-工作站锁定"触发器重复
    <Query Id='6' Path='Security'>
        <Select Path='Security'>
            *[System[Provider[@Name='Microsoft-Windows-Security-Auditing'] and EventID=4800]]
        </Select>
    </Query> -->
    <!-- 用户注销 4634 / 4647 -->
    <!-- <Query Id="6" Path="Security">
        <Select Path="Security">
            *[System[(EventID=4800 or EventID=4634 or EventID=4647)]]
        </Select>
    </Query> -->

    <!-- 查询 7（已停用）：系统日志（System）里的 Winlogon 通知事件。用户登录（会话连接）—— 事件 ID 7001。
         停用原因：与"登录时"触发器语义重复
    <Query Id='7' Path='System'>
        <Select Path='System'>
            *[System[Provider[@Name='Microsoft-Windows-Winlogon'] and EventID=7001]]
        </Select>
    </Query> -->

    <!-- 查询 8（已停用）：用户注销/断开 —— 事件 ID 7002。
         停用原因：注销/断开时用户已离开桌面，无需刷新壁纸
    <Query Id='8' Path='System'>
        <Select Path='System'>
            *[System[Provider[@Name='Microsoft-Windows-Winlogon'] and EventID=7002]]
        </Select>
    </Query> -->
</QueryList>"#;

// 创建Windows任务计划:
// https://docs.microsoft.com/zh-cn/windows/win32/taskschd/task-scheduler-start-page
// https://learn.microsoft.com/zh-cn/windows/win32/api/_taskschd
// 解锁、启动、登录等事件触发任务计划 taskschd.msc
// 禁用日志：wevtutil sl Microsoft-Windows-TaskScheduler/Operational /e:false
pub fn create_schedule() -> Result<(), Box<dyn std::error::Error>> {
    // 获取当前执行程序的路径
    let exe_path = env::current_exe()?;
    let exe_path_str = exe_path
        .to_str()
        .ok_or_else(std::io::Error::last_os_error)?;

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
            // principal.SetId()?;
            // principal.SetDisplayName()?;
            // 运行账户 (User/Group): 默认为当前用户
            // principal.SetUserId(&BSTR::from("SYSTEM"))?;

            // 登录选项 (SetLogonType)
            // 可选值：
            // TASK_LOGON_NONE 0 未指定登录方式，用于非 NT 凭据
            // TASK_LOGON_PASSWORD 1 用密码登录指定用户
            //      对应UI界面上的 不管用户是否登录都要运行。
            // TASK_LOGON_S4U 2 Service For User (S4U) 登录，非交互式
            //      对应UI界面上的 不管用户是否登录都要运行。
            //      不存储密码。该任务将只有访问本地资源的权限
            // TASK_LOGON_INTERACTIVE_TOKEN 3 必须用户已登录，在已有交互会话中运行
            //      对应UI界面上的 只在用户登录时运行。
            // TASK_LOGON_GROUP 4 组激活：类似 INTERACTIVE_TOKEN，但组内任意成员均可触发
            // TASK_LOGON_SERVICE_ACCOUNT 5 使用本地系统账号（LocalSystem/LocalService/NetworkService）
            // TASK_LOGON_INTERACTIVE_TOKEN_OR_PASSWORD 6 优先用交互令牌，没有则用密码（新系统上基本等同 PASSWORD）
            // 壁纸程序必须运行在用户交互桌面会话：
            // SystemParametersInfo(SPI_SETDESKWALLPAPER) 与 IDesktopWallpaper 均为 per-session 桌面资源，
            // 写入当前会话的 HKCU\Control Panel\Desktop\Wallpaper 并向当前桌面广播 WM_SETTINGCHANGE。
            // S4U/PASSWORD（"不管是否登录都运行"）会落到 Session 0 非交互会话，壁纸设置对可见桌面无效，
            // 表现为登录/解锁后壁纸不变。故必须用 INTERACTIVE_TOKEN（"只在用户登录时运行"），
            // 任务才能进入用户桌面会话正确设置壁纸。
            let logon_type = TASK_LOGON_INTERACTIVE_TOKEN;
            // 这里与调用RegisterTaskDefinition注册任务时传值作用一致
            principal.SetLogonType(logon_type)?;

            // principal.SetGroupId()?;

            // 使用最高权限运行 (Run with highest privileges)
            // 可选值：
            // TASK_RUNLEVEL_LUA: 标准用户权限
            // TASK_RUNLEVEL_HIGHEST: 管理员/最高权限
            principal.SetRunLevel(TASK_RUNLEVEL_HIGHEST)?;

            let settings: ITaskSettings = task_definition.Settings()?;

            // 隐藏 (Hidden)
            settings.SetHidden(VARIANT_FALSE)?;

            // 配置 (Configure for) / 兼容性设置 (SetCompatibility)
            // 可选值：
            // TASK_COMPATIBILITY_V1: Windows Server 2003, Windows XP, or Windows 2000
            // TASK_COMPATIBILITY_V2: Windows Vista, Windows Server 2008
            // TASK_COMPATIBILITY_V2_1: Windows 7, Windows Server 2008 R2
            // TASK_COMPATIBILITY_V2_2: Windows 8
            // TASK_COMPATIBILITY_V2_3: Windows 8.1
            // TASK_COMPATIBILITY_V2_4: Windows 10
            settings.SetCompatibility(TASK_COMPATIBILITY_V2_4)?;

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
            i_once_trigger.SetRandomDelay(&BSTR::from("PT10S"))?;
            // 【已停用】禁用触发器的重复设置：无 Duration 的 Repetition 即无限重复，且禁用
            // 触发器残留 Repetition 配合 StartWhenAvailable 会被统一引擎按"错失排期补偿"
            // 持续播种——实测带过去 StartBoundary 的 Time/Calendar 类必现每 5 分钟僵尸
            // 自动运行，删除重建任务、重启任务计划服务均无法清除，唯有去掉 Repetition
            /*let rep_once = i_once_trigger.Repetition()?;
            // 重复任务间隔
            rep_once.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            // rep_once.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_once.SetStopAtDurationEnd(VARIANT_FALSE)?;*/
            // 运行时间超过此值则停止执行
            i_once_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 过期时间 (Expire)
            // i_once_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
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
            // 【已停用】禁用触发器的重复设置（同上：残留 Repetition 会形成僵尸自动运行）
            /*let rep_daily = i_daily_trigger.Repetition()?;
            // 重复任务间隔
            rep_daily.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_daily.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_daily.SetStopAtDurationEnd(VARIANT_FALSE)?;*/
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
            i_weekly_trigger.SetRandomDelay(&BSTR::from("PT10S"))?;
            // 【已停用】禁用触发器的重复设置（同上：残留 Repetition 会形成僵尸自动运行）
            /*let rep_weekly = i_weekly_trigger.Repetition()?;
            // 重复任务间隔
            rep_weekly.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            // rep_weekly.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_weekly.SetStopAtDurationEnd(VARIANT_FALSE)?;*/
            // 运行时间超过此值则停止执行
            i_weekly_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 过期时间 (Expire)
            // i_weekly_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
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
            i_monthly_trigger.SetRandomDelay(&BSTR::from("PT10S"))?;
            // 【已停用】禁用触发器的重复设置（同上：残留 Repetition 会形成僵尸自动运行）
            /*let rep_monthly = i_monthly_trigger.Repetition()?;
            // 重复任务间隔
            rep_monthly.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            // rep_monthly.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_monthly.SetStopAtDurationEnd(VARIANT_FALSE)?;*/
            // 运行时间超过此值则停止执行
            i_monthly_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 过期时间 (Expire)
            // i_monthly_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
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
            i_mdow_trigger.SetRandomDelay(&BSTR::from("PT10S"))?;
            // 【已停用】禁用触发器的重复设置（同上：残留 Repetition 会形成僵尸自动运行）
            /*let rep_mdow = i_mdow_trigger.Repetition()?;
            // 重复任务间隔
            rep_mdow.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            // rep_mdow.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_mdow.SetStopAtDurationEnd(VARIANT_FALSE)?;*/
            // 运行时间超过此值则停止执行
            i_mdow_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 过期时间 (Expire)
            // i_mdow_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
            // 启用
            i_mdow_trigger.SetEnabled(VARIANT_FALSE)?;

            // --- 2.6 登录时 ---
            let t_logon = triggers.Create(TASK_TRIGGER_LOGON)?;
            let i_logon_trigger: ILogonTrigger = t_logon.cast()?;
            // 特定用户 (如果不设置则为“所有用户”)
            // i_logon_trigger.SetUserId(&BSTR::from("DOMAIN\\User"))?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_logon_trigger.SetDelay(&BSTR::from("PT1M"))?;
            /*let rep_logon = t_logon.Repetition()?;
            // 重复任务间隔
            rep_logon.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            rep_logon.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_logon.SetStopAtDurationEnd(VARIANT_FALSE)?;
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
            rep_boot.SetStopAtDurationEnd(VARIANT_FALSE)?;
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
            // 【已停用】禁用触发器的重复设置（实测无 StartBoundary 者不复燃，但同属
            // 禁用触发器携带 Repetition 的隐患类别，一并停用）
            /*let rep_idle = i_idle_trigger.Repetition()?;
            // 重复任务间隔
            rep_idle.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            // rep_idle.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_idle.SetStopAtDurationEnd(VARIANT_FALSE)?;*/
            // 运行时间超过此值则停止执行
            i_idle_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            // i_idle_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            // i_idle_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
            // 启用
            i_idle_trigger.SetEnabled(VARIANT_FALSE)?;

            // --- 2.9 发生事件时 ---
            let t_event = triggers.Create(TASK_TRIGGER_EVENT)?;
            let i_event_trigger: IEventTrigger = t_event.cast()?;
            // 基本设置 (通过 XML 定义筛选器)
            // 定义事件查询。触发器将启动任务，当收到事件时。
            // 订阅内容提取至文件头 EVENT_SUBSCRIPTION_XML 常量（便于测试校验结构合法性）
            i_event_trigger.SetSubscription(&BSTR::from(EVENT_SUBSCRIPTION_XML))?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_event_trigger.SetDelay(&BSTR::from("PT10S"))?;
            // 【已停用】事件触发器的重复设置：Repetition 未设置 Duration 时语义为"无限重复"——
            // 唤醒事件命中一次后任务将从此每 5 分钟重复执行，与"每次唤醒仅触发一次刷新壁纸"的
            // 设计意图相悖，也违背进程防堆积原则（仅剩单实例锁兜底），故停用
            /*let rep_event = i_event_trigger.Repetition()?;
            // 重复任务间隔
            rep_event.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            // rep_event.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_event.SetStopAtDurationEnd(VARIANT_FALSE)?;*/
            // 运行时间超过此值则停止执行
            i_event_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            // i_event_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            // i_event_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
            // 启用
            i_event_trigger.SetEnabled(VARIANT_TRUE)?;

            // --- 2.10 创建/修改任务时 ---
            let t_reg = triggers.Create(TASK_TRIGGER_REGISTRATION)?;
            let i_reg_trigger: IRegistrationTrigger = t_reg.cast()?;
            // --- 高级设置 (Advanced Settings) ---
            // 任务最多延迟时间
            i_reg_trigger.SetDelay(&BSTR::from("PT10S"))?;
            // 【已停用】禁用触发器的重复设置（实测无 StartBoundary 者不复燃，但同属
            // 禁用触发器携带 Repetition 的隐患类别，一并停用）
            /*let rep_reg = i_reg_trigger.Repetition()?;
            // 重复任务间隔
            rep_reg.SetInterval(&BSTR::from("PT5M"))?;
            // 持续时间
            // rep_reg.SetDuration(&BSTR::from("PT1H"))?;
            // 重复持续时间结束时停止所有运行的任务
            rep_reg.SetStopAtDurationEnd(VARIANT_FALSE)?;*/
            // 运行时间超过此值则停止执行
            i_reg_trigger.SetExecutionTimeLimit(&BSTR::from("PT2H"))?;
            // 激活 (StartBoundary)
            // i_reg_trigger.SetStartBoundary(&BSTR::from("2024-01-01T00:00:00"))?;
            // 过期时间 (Expire)
            // i_reg_trigger.SetEndBoundary(&BSTR::from("2024-12-31T23:59:59"))?;
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
            rep_tssc_cc.SetStopAtDurationEnd(VARIANT_FALSE)?;
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
            rep_tssc_cd.SetStopAtDurationEnd(VARIANT_FALSE)?;
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
            rep_tssc_rc.SetStopAtDurationEnd(VARIANT_FALSE)?;
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
            rep_tssc_rd.SetStopAtDurationEnd(VARIANT_FALSE)?;
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
            rep_tssc_sl.SetStopAtDurationEnd(VARIANT_FALSE)?;
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
            rep_tssc_su.SetStopAtDurationEnd(VARIANT_FALSE)?;
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
            // --scheduled：程序据此进入任务计划静默模式（出错不弹窗、以非零码退出），
            // 避免任务场景无人点击阻塞弹窗导致进程挂死堆积
            exec_action.SetArguments(&BSTR::from("--scheduled"))?;
            // 起始于 (Start in)
            if let Some(parent) = exe_path.parent()
                && let Some(dir_str) = parent.to_str() {
                    exec_action.SetWorkingDirectory(&BSTR::from(dir_str))?;
                }

            // ==========================================
            // 4. 条件 (Conditions)
            // ==========================================

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
            // 尝试重新启动最多次数（原 3 次会在触发风暴时把失败实例放大 4 倍，收敛为 1 次）
            settings.SetRestartCount(1)?;
            // 如果任务运行时间超过以下时间，停止任务
            settings.SetExecutionTimeLimit(&BSTR::from("PT10M"))?;
            // 如果请求后任务还在运行，强行将其停止。
            // 必须为 TRUE：控制台程序收不到软终止（WM_CLOSE），FALSE 会让超时任务无法被
            // 任务计划回收（如失败弹窗挂住的进程），进程只增不减直至系统卡顿
            settings.SetAllowHardTerminate(VARIANT_TRUE)?;
            // 如果任务没有计划再次运行，则在此之后删除该任务
            // settings.SetDeleteExpiredTaskAfter(&BSTR::from("PT0S"))?;
            // 如果此任务已经运行，以下规则适用 (Multiple Instances)
            // 可选值：
            // TASK_INSTANCES_PARALLEL: 并行运行新实例
            // TASK_INSTANCES_QUEUE: 对新实例排队
            // TASK_INSTANCES_IGNORE_NEW: 请勿启动新实例
            // TASK_INSTANCES_STOP_EXISTING: 停止现有实例
            // 必须为 IGNORE_NEW：PARALLEL 下每次触发都无条件并行拉起新进程，
            // 高频触发器（全量事件/全量登录）叠加会瞬间产生大量实例形成进程风暴；
            // IGNORE_NEW 保证任意时刻至多一个实例运行，是系统级的第一道防线
            settings.SetMultipleInstances(TASK_INSTANCES_IGNORE_NEW)?;

            // Windows 7+
            let settings2: ITaskSettings2 = settings.cast()?;
            // 控制任务是否可以在 RemoteApp (RAIL) 会话中启动
            settings2.SetDisallowStartOnRemoteAppSession(VARIANT_FALSE)?;
            // 是否使用统一调度引擎（Unified Scheduling Engine，性能更好、更现代的引擎）
            // 统一调度引擎是 Windows 7 及更高版本引入的一种机制，旨在让“任务计划程序”和“Windows 服务”的行为更加一致，提高性能
            // 【已停用】统一调度引擎为维护任务配套引入的设置；触发式壁纸任务走任务计划程序
            // 默认引擎即可，减少非常规引擎带来的不确定调度行为
            // settings2.SetUseUnifiedSchedulingEngine(VARIANT_TRUE)?;

            // Windows 8+
            // 【已停用】维护设置（自动维护任务化）：CreateMaintenanceSettings 一旦生效，任务计划
            // 会将本任务交由"自动维护"调度器管理——仅在系统空闲且使用交流电源的机会性维护窗口内
            // 运行，用户使用电脑时还会被推迟/挂起；常规触发器（登录/启动/事件/会话解锁）不再被
            // 正常调度，实测表现为排期持续滚动却全部记为 Missed、从不启动。
            // 壁纸任务恰恰要在用户唤醒/回到桌面时更换，与维护任务"用户离开才跑"的语义根本对立，
            // 故整体停用（保留代码备查）
            /*if let Ok(settings3) = settings.cast::<ITaskSettings3>() {
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
            }*/

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
                    &VARIANT::default(), // UserID
                    &VARIANT::default(), // &empty_var // Password
                    logon_type,          // LogonType
                    &VARIANT::default(), // sddl
                )?;
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

/// 带超时保护的 create_schedule 入口
///
/// 任务计划服务（Schedule）被大量任务实例冲击乃至挂死时，RegisterTaskDefinition 等
/// 阻塞 COM 调用会永不返回（服务卡死连 net stop 都无响应）；直接调用 create_schedule
/// 会让进程永久挂起。此包装在独立线程执行注册流程，主线程限时等待：
/// 服务 30 秒内无响应即返回带处置指引的错误，进程得以继续退出。
///
/// 形参：无（透传 create_schedule）
pub fn create_schedule_guarded() -> Result<(), Box<dyn std::error::Error>> {
    match
        // Box<dyn Error> 非线程安全（Send），跨线程传递前在工作线程内降级为 String
        utils::run_with_timeout(Duration::from_secs(30), || {
            create_schedule().map_err(|e| e.to_string())
        }) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e.into()),
        Err(timeout_err) => Err(format!(
            "任务计划服务无响应（{timeout_err}）。服务可能正被大量任务实例冲击，请重启电脑后重试"
        )
        .into()),
    }
}

/// 注册开机启动项：写入注册表 `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`
///
/// 语义与原 winreg 版一致：子键不存在则创建、存在则打开（RegCreateKeyExW），
/// 值不存在则写入、存在则覆盖，值类型 REG_SZ。写 HKLM 需要管理员权限——
/// 本程序 manifest 已声明 requireAdministrator，任务计划亦以最高权限运行。
///
/// 形参 `app_name`：注册表值名称，即开机启动项的条目名（通常为程序名）
/// 形参 `app_path`：开机自启程序的完整路径
pub fn add_to_startup(
    app_name: &str,
    app_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Win32 注册表 API 均为宽字符串：子键路径/值名转 UTF-16 并补 null 终止符
    let subkey: Vec<u16> = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let value_name: Vec<u16> = app_name.encode_utf16().chain(std::iter::once(0)).collect();
    // REG_SZ 数据本身也必须以 null 结尾，按 UTF-16 小端字节序列传入（cbData 由 API 自动计算）
    let value_data: Vec<u8> = app_path
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect();

    unsafe {
        let mut hkey = HKEY::default();
        // 创建或打开子键：REG_OPTION_NON_VOLATILE 持久化存储，KEY_WRITE 涵盖创建子键与设值权限
        let create_result = RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        );
        if create_result != ERROR_SUCCESS {
            return Err(hresult_error(create_result));
        }

        // 写入/覆盖值；无论成败都先释放句柄再返回，避免句柄泄漏
        let set_result = RegSetValueExW(
            hkey,
            PCWSTR(value_name.as_ptr()),
            None,
            REG_SZ,
            Some(&value_data),
        );
        let close_result = RegCloseKey(hkey);
        if set_result != ERROR_SUCCESS {
            return Err(hresult_error(set_result));
        }
        if close_result != ERROR_SUCCESS {
            return Err(hresult_error(close_result));
        }
    }

    Ok(())
}

/// 将 WIN32_ERROR 包装为带系统描述文本的错误（注册表写入辅助）
///
/// 形参 `code`：Win32 注册表 API 返回的错误码
fn hresult_error(code: WIN32_ERROR) -> Box<dyn std::error::Error> {
    windows::core::Error::from_hresult(HRESULT::from_win32(code.0))
        .to_string()
        .into()
}

#[cfg(test)]
mod tests {
    use super::EVENT_SUBSCRIPTION_XML;

    /// 去除 XML 注释，返回仅含活动节点的文本（测试辅助）
    ///
    /// `xml`：待处理的订阅 XML 文本
    fn strip_xml_comments(xml: &str) -> String {
        let mut active = String::with_capacity(xml.len());
        let mut rest = xml;
        while let Some(start) = rest.find("<!--") {
            active.push_str(&rest[..start]);
            let Some(end) = rest[start + 4..].find("-->") else {
                // 注释未闭合：视为其余内容全部处于注释内
                rest = "";
                break;
            };
            rest = &rest[start + 4 + end + 3..];
        }
        active.push_str(rest);
        active
    }

    /// 停用的洪水查询不得以活动节点形式存在于订阅中：
    /// 任一查询保持活动都会在短时间内高频触发任务，形成进程风暴
    #[test]
    fn should_not_contain_active_flood_queries() {
        let active = strip_xml_comments(EVENT_SUBSCRIPTION_XML);
        assert!(!active.contains("Level=2"), "System 全量错误查询必须停用");
        assert!(!active.contains("EventID=4624"), "4624 全量登录查询必须停用");
        assert!(!active.contains("EventID=4800"), "锁定查询必须停用");
        assert!(!active.contains("EventID=4801"), "解锁查询必须停用");
        assert!(!active.contains("EventID=7001"), "Winlogon 登录查询必须停用");
        assert!(!active.contains("EventID=7002"), "Winlogon 注销查询必须停用");
    }

    /// 保留的睡眠恢复查询必须保持活动状态：
    /// 每次睡眠/休眠唤醒仅触发一次，用于唤醒后刷新壁纸，频率低且合理
    #[test]
    fn should_keep_sleep_resume_query_active() {
        let active = strip_xml_comments(EVENT_SUBSCRIPTION_XML);
        assert!(
            active.contains("Power-Troubleshooter"),
            "睡眠恢复查询必须保留"
        );
        assert!(active.contains("EventID=1"), "睡眠恢复事件 ID 必须保留");
    }

    /// 注释开闭定界符必须数量一致：
    /// 不配对意味着订阅 XML 结构损坏，SetSubscription 注册的触发器会失效或注册失败
    #[test]
    fn should_have_paired_comment_delimiters() {
        assert_eq!(
            EVENT_SUBSCRIPTION_XML.matches("<!--").count(),
            EVENT_SUBSCRIPTION_XML.matches("-->").count(),
            "注释开闭定界符必须数量一致"
        );
    }
}
