# 项目 AI 协作规范（desktop-wallpaper-rust）

Windows 桌面壁纸自动更换程序：任务计划触发 → 下载壁纸 → 设置桌面。技术栈 Rust + tokio + windows-rs 0.62 + winresource。

## 1. 架构约定

### 1.1 Windows 后台任务防堆积三件套（强制）
防止"进程不停新增导致系统卡顿"的三道防线，**三者必须同时存在**，禁止单独依赖任务计划策略：
1. 任务计划多实例策略 = "请勿启动新实例"（`TASK_INSTANCES_IGNORE_NEW`，task_scheduler.rs）
2. 程序内单实例锁（`utils::try_acquire_single_instance`，`CreateMutexW` + `Local\` 会话命名空间）
3. 程序内运行时长自限（main 的 `tokio::time::timeout(EXECUTION_TIME_LIMIT)`，与任务计划 `PT10M` 数值对齐）

### 1.2 无人值守场景禁止阻塞 UI（强制）
- 任务计划启动（`--scheduled` 参数）的进程**严禁弹窗**：无人点击的 MessageBox 会挂死进程
  → 出错路径：写 stderr + `std::process::exit(1)`（退出码记录在任务计划历史）
- 仅手动交互运行允许 MessageBox；新增任何错误处理必须先区分 `is_scheduled`
- 任务计划的 Action 参数固定为 `--scheduled`（task_scheduler.rs），手动运行不带参数
- **弹窗文本生命周期规则**：文本传入 `MSGBOXPARAMSW` 等结构体前必须绑定局部
  `HSTRING` 变量；结构体字段内联 `HSTRING::from(x).as_ptr()` 的临时值在 `let` 语句
  结束即释放，MessageBox 读取悬垂指针导致内容乱码（`MessageBoxW` 参数位置的
  内联临时值存活到调用结束，属安全写法）

### 1.3 阻塞 FFI 必须限时防御（强制）
- 任务计划服务的 COM 调用（RegisterTaskDefinition 等）是阻塞 FFI：服务被任务实例
  冲击乃至挂死时调用**永不返回**（连 `net stop schedule` 都无响应）
- **tokio 超时打不断阻塞 FFI**：`tokio::time::timeout` 到期后内部 future 会被丢弃，
  但卡在 FFI 里的线程不会返回，runtime 收尾时无限等待 → 进程挂死
- 唯一可靠的程序级防御：`utils::run_with_timeout`（旁路线程执行 + 主线程
  `recv_timeout` 限时等待；超时以线程泄漏换取进程存活）
- 跨线程传递错误：`Box<dyn Error>` 非 `Send`，须在工作线程内降级为 `String`
- 所有阻塞 FFI 调用（任务计划 COM、注册表、未来任何系统服务调用）必须经
  `run_with_timeout` 包装；错误路径必须 `process::exit` 终止（含卡死线程在内的全部线程）

### 1.4 网络请求统一客户端（强制）
- 所有 HTTP 请求必须经 `images::http_client()`（浏览器 UA + 30s 超时）
- **禁止**裸用 `reqwest::get` / `Client::new()`——默认无超时，网络异常时进程无限挂起

### 1.5 版本号单一来源
- exe 版本资源（File/ProductVersion 及数字版块）由 build.rs 的 winresource 默认取自
  `Cargo.toml [package].version`，**升版只改 Cargo.toml**
- 资源字符串字段（FileDescription/ProductName 等）在 build.rs 的 `res.set(...)` 维护，
  资源语言 0x0804 简体中文

### 1.6 任务计划触发器约束
- 事件订阅 XML 集中在 `task_scheduler::EVENT_SUBSCRIPTION_XML` 常量（便于测试校验）
- 高频/全量事件查询（System 全量错误、4624 全量登录、Winlogon 7001/7002、4800/4801）
  已**注释停用**（未删除）；恢复启用前必须确认 1.1 三件套全部就位
- 登录/解锁场景由任务级"登录时""会话更改"触发器覆盖，勿加冗余事件查询

### 1.7 任务计划注册设置红线（平台行为实测，2026-09-11）
- **严禁 MaintenanceSettings（维护任务化）**：`CreateMaintenanceSettings`（Period/Deadline）
  一旦设置，任务被"自动维护"调度器接管——仅在"空闲 + 交流电源"窗口运行，常规触发器
  （登录/启动/事件/会话解锁）全部被搁置。典型症状：`MissedRuns` 暴涨、`LastRunTime`
  冻结、`NextRunTime` 持续滚动排期但从不启动。触发式壁纸任务与"用户离开才跑"的维护
  语义根本对立（task_scheduler.rs 中已注释停用，保留代码备查）
- **任何触发器严禁 Repetition 无 Duration**：无 Duration 即无限重复；且**禁用触发器**
  （带过去时间 StartBoundary）残留 Repetition 时，配合 `StartWhenAvailable` 会被统一
  调度引擎按"错失排期补偿"持续播种——形成每 5 分钟僵尸自动运行，**删除重建任务、
  重启任务计划服务均无法清除**（每次注册/重启都从 XML 重新播种），唯有从定义中移除
  Repetition。恢复任一触发器的重复设置前，必须明确设置有限 Duration 并复测
- **禁用触发器的未来日历排期仅显示不执行**：`NextRunTime` 会显示禁用 Weekly/Monthly
  等触发器的理论排期（如某周一 08:00），探针实测到点不启动，属显示层现象，无需处理
- **UseUnifiedSchedulingEngine 在 Win11 为系统常态**：本机 200 个任务中 187 个为 true；
  代码注释停用该设置后注册出的 XML 仍显示 true 属正常，非回归
- **触发层问题诊断法**：`Get-ScheduledTask -TaskName SetBingWallpaper | Get-ScheduledTaskInfo`
  观察 MissedRuns/NextRunTime 是否滞涨；`TaskScheduler/Operational` 事件链
  107（触发）→100（启动）→129（PID）→201（完成）定位断点，322=IgnoreNew 拒并发。
  历史日志默认关闭，开启：`wevtutil sl Microsoft-Windows-TaskScheduler/Operational /e:true`

## 2. 黄金回归门禁（Golden Master，每次改动后执行）

1. `cargo build --release` → 零 error 零 warning
2. 4 个单元测试通过（测试 exe 含 requireAdministrator manifest，需 UAC 提权运行）：
   - `task_scheduler::tests` ×3：订阅 XML 结构校验（洪水查询停用、睡眠查询保留、注释定界符配对）
   - `utils::tests` ×1：单实例锁同进程二次抢占被拒
3. exe 版本资源 8 字段回归（PowerShell 读 `VersionInfo`，UTF-8 输出文件核对中文）
4. `--scheduled` 双实例冒烟：第二实例 <1s 内以退出码 0 退出（单实例锁生效）
5. 任务计划注册后 XML 校验（`Export-ScheduledTask -TaskName SetBingWallpaper`）：
   无 `MaintenanceSettings`、无任何 `Repetition` 元素、`NextRunTime` 无固定周期排期
   （僵尸排期特征，判定标准见 1.7）

## 3. 已知技术债（用户决策保留现状，勿在其上叠加新功能）

- `utils::is_connected`、`utils::set_lock_screen_wallpaper` 为零调用死代码；
  `is_connected` 的五重网络探测存在变量遮蔽（实际仅第 5 种 GetAdaptersInfo 生效），
  其中 `_is_alive` 探测有真实网络 I/O 副作用，严禁删除语句
- `utils::set_wallpaper` 的 `Ok(true)` 成功分支返回布尔被上层丢弃（无害冗余，待后续理顺）
- **平台行为知识**：`SystemParametersInfoW(SPI_SETDESKWALLPAPER)` 对不存在的文件路径
  **不校验、直接返回成功**（仅写注册表并广播，图片加载由 Explorer 静默失败）——
  set_wallpaper 已加前置存在性校验拦截，`ACCESS_DENIED` 亦已显式上抛；
  两者由 `should_return_error_when_wallpaper_cannot_be_set` 测试固化，勿回退

## 4. 开发工作流

- 架构师统筹 + 高级开发工程师子 Agent 并行开发；任务派发前先 grep 验证依赖/函数真实引用情况
- 任务涉及同一文件时**必须串行**（如 Cargo.toml、utils.rs），无交集才可并行
- TDD 适用场景：错误路径、纯函数逻辑（先写失败测试）；机械替换类用 2 的回归门禁兜底
