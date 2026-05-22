# desktop-wallpaper-rust

设置桌面壁纸 rust实现


**使用**

```pwsh
desktop-wallpaper-rust.exe -t --taskschd
```

**编译**

```bash
cargo tree --edges=features --package= --invert
# MSVC
cargo rustc --release -- -Clink-args="/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup"
# GCC 
cargo rustc --release -- -Clink-args="-Wl,--subsystem,windows"
```

**窗口相关**

| API                   | 说明                                              |
| --------------------- | ----------------------------------------------- |
| `MessageBoxExW`       | 支持额外语言区域标识（`wLanguageId`）                       |
| `MessageBoxIndirectW` | 支持更丰富的自定义（图标、按钮、帮助 ID 等），通过 `MSGBOXPARAMSW` 结构体 |
| `MessageBeep`         | 只播放提示音，不显示对话框                                   |
| `ShellMessageBoxW`                  | Shell32 版本的消息框                    |
| `SHMessageBoxCheckW`                | 带"不再询问"复选框                        |
| `TaskDialog` / `TaskDialogIndirect` | Vista+ 的现代任务对话框（更美观，支持进度条、自定义按钮等） |
| `NotifyUser` / `Shell_NotifyIconW`  | 系统托盘气泡通知                          |
| `FlashWindow` / `FlashWindowEx`     | 窗口闪烁提醒                            |


**官方文档**

* [https://learn.microsoft.com/zh-cn/windows/apps/desktop/modernize/winrt-apis-desktop-apps](https://learn.microsoft.com/zh-cn/windows/apps/desktop/modernize/winrt-apis-desktop-apps)
* WinRT 语言投影（language projection） [https://github.com/microsoft/xlang](https://github.com/microsoft/xlang)
    * [https://github.com/microsoft/win32metadata](https://github.com/microsoft/win32metadata)
    * [https://github.com/microsoft/cppwinrt](https://github.com/microsoft/cppwinrt)
        * [https://github.com/kennykerr/win7](https://github.com/kennykerr/win7)
    * [https://github.com/microsoft/windows-rs](https://github.com/microsoft/windows-rs)
        * [https://github.com/kennykerr/blog](https://github.com/kennykerr/blog)
        * [https://microsoft.github.io/windows-docs-rs/doc/windows](https://microsoft.github.io/windows-docs-rs/doc/windows)
        * [https://docs.rs/crate/windows/latest](https://docs.rs/crate/windows/latest)
    * TaskScheduler [https://github.com/mattrobineau/planif](https://github.com/mattrobineau/planif)
    * [https://github.com/retep998/winapi-rs](https://github.com/retep998/winapi-rs)
    * [https://github.com/rodrigocfd/winsafe](https://github.com/rodrigocfd/winsafe)
    * [https://github.com/mxre/winres](https://github.com/mxre/winres)
    * [https://github.com/Araxeus/tiny-native-scheduler](https://github.com/Araxeus/tiny-native-scheduler)
    * [https://github.com/microsoft/cswinrt](https://github.com/microsoft/cswinrt)
    * [https://github.com/pywinrt/pywinrt](https://github.com/pywinrt/pywinrt)
    * [https://github.com/NodeRT/NodeRT](https://github.com/NodeRT/NodeRT)
    * [https://github.com/alvinhochun/mingw-w64-cppwinrt](https://github.com/alvinhochun/mingw-w64-cppwinrt)
    * [https://github.com/hez2010/WinRTServer](https://github.com/hez2010/WinRTServer)



## 自动运行

- 服务 `services.msc` 登录时启动，手动启动
- 计划任务 `taskschd.msc` 可以做到更复杂的触发
    - 按预定计划：一次(N)、每天(D)、每周(W)、每月(M)
    - 登录时
    - 启动时
    - 空闲状态
    - 发生事件时
    - 创建/修改任务时
    - 当连接到用户会话时
    - 当从用户会话断开连接时
    - 工作站锁定时
    - 工作站解锁时
- 登录时启动
    - 系统运行 `计算机\HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`
    - 系统开始菜单 `C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Startup`
    - 用户运行 `计算机\HKEY_USERS\xxx\Software\Microsoft\Windows\CurrentVersion\Run`
    - 用户开始菜单 `%AppData%\Microsoft\Windows\Start Menu\Programs\Startup`




### 一、 创建基本任务 (Basic Task) - 向导模式

这是为普通用户设计的简化流程，只包含核心三要素：

1.  **名称和描述**：
    * 名称 (Name)
    * 描述 (Description)
2.  **触发器 (Triggers)**：
    * 每天
        * 开始时间
        * 每隔多少天发生一次
    * 每周
        * 开始时间
        * 每隔多少周星期几
    * 每月
        * 开始时间
        * 月
        * 天
        * 在第几个星期几
    * 一次
        * 开始时间
    * 计算机启动时
    * 当前用户登录时
    * 当特定事件被记录时
        * 日志
        * 源
        * 事件ID
3.  **操作 (Actions)**：
    * 启动程序 (Start a program)
        * 程序或脚本
        * 添加参数
        * 起始于



### 二、 创建任务 (Create Task) - 完整模式

#### 1. 常规 (General)

* 名称 (Name): `String`
* 位置 (Location): 任务所在的文件夹路径，默认`\`
* 作者 (Author): 默认当前用户。
* 描述 (Description): `String`
* 安全选项 (Security Options):
    * 运行账户: `User/Group` (默认为当前用户)。
    * 单选: “只在用户登录时运行” 或 “不管用户是否登录都要运行”。
    * 不存储密码 (Do not store password): 仅在访问本地资源时有效（S4U）。
    * 使用最高权限运行 (Run with highest privileges): 是否申请管理员/SYSTEM权限。
    * 隐藏 (Hidden): UI 中不可见。
    * 配置 (Configure for)**: 兼容性设置
        * Windows 10
        * Windows® 7,Windows Server™ 2008 R2
        * Windows Vista™、Windows ServerT 2008
        * Windows ServerTM 2003、Windows®XP或Windows® 2000

#### 2. 触发器 (Triggers)

一个任务可以有**多个**触发器。每个触发器包含：

* **开始任务 (Begin the task)**:
    * 按预定计划
        * 一次(N)
            * 开始时间
        * 每天(D)
            * 开始时间
            * 每隔多少天发生一次
        * 每周(W)
            * 开始时间
            * 每隔多少周星期几
        * 每月(M)
            * 开始时间
            * 月
            * 天
            * 在第几个星期几
    * 登录时
        * 所有用户
        * 特定用户
    * 启动时
        * 不需要其他设置。
    * 空闲状态
        * 若要更改空闲条件，请使用“创建任务”或任务“属性”页中的“条件”页面。
    * 发生事件时
        * 基本
            * 日志
            * 源
            * 事件ID
        * 自定义
            * 新增事件筛选器
                * 筛选器
                    * 记录时间
                    * 事件级别：关键、警告、详细、错误、信息
                    * 按日志 事件日志：Windows日志、应用程序和服务日志
                    * 按源 事件来源：
                    * 任务类别
                    * 关键字
                    * 用户
                    * 计算机
                * XML
    * 创建/修改任务时
        * 不需要其他设置。
    * 当连接到用户会话时
        * 所有用户
        * 特定用户
        * 远程计算机的连接(O)
        * 本地计算机的连接(N)
    * 当从用户会话断开连接时
        * 所有用户
        * 特定用户
        * 远程计算机的连接(O)
        * 本地计算机的连接(N)
    * 工作站锁定时
        * 所有用户
        * 特定用户
    * 工作站解锁时
        * 所有用户
        * 特定用户
* **高级设置 (Advanced settings)**:
    * 延迟任务时间 (Delay task for): `Duration`，空闲状态类型不可选，按预定计划类型名称为“任务最多延迟时间(随机延迟)(K)”
    * 重复任务间隔 (Repeat task every): `Duration` (例如每5分钟)。
        * 持续时间 (for a duration of): 重复多久。
        * 重复持续时间结束时停止所有运行的任务: `bool`
    * 任务的运行时间超过此值则停止执行
    * 激活：`DateTime`，按预定计划类型没有此项
    * 到期时间 (Expire): `DateTime`
    * 启用 (Enabled): `bool`


#### 3. 操作 (Actions)

一个任务可以有**多个**操作（顺序执行）。

* 操作类型: 启动程序 (Start a program)。
    * 程序/脚本 (Program/script): 可执行文件路径。
    * 添加参数 (Add arguments): `String`
    * 起始于 (Start in): 工作目录。


#### 4. 条件 (Conditions)

* **空闲 (Idle)**:
    * 仅当计算机空闲时间超过下列值时才启动此任务：`Duration`
        * 等待空闲时间：`Duration`
        * 如果计算机不再空闲，则停止(E)
        * 如果空闲状态继续，则重新启动(U)
* **电源 (Power)**:
    * 只有在计算机使用交流电源时才启动此任务(P)
        * 如果计算机改用电池电源，则停止(B)
    * 唤醒计算机运行此任务 (Wake the computer)。
* **网络 (Network)**:
    * 只有在以下网络连接可用时才启动：任何连接、网络

#### 5. 设置 (Settings)

* 允许按需运行
* 如果过了计划开始时间，立即启动任务(S)
* 如果任务失败，按以下频率重新启动： 1分钟、5分钟、10分钟、15分钟、30分钟、1小时、2小时
    * 尝试重新启动最多次数(R)
* 如果任务运行时间超过以下时间，停止任务:默认 3 天。
* 如果请求后任务还在运行，强行将其停止
* 如果任务没有计划再次运行，则在此之后删除该任务(D):立即、30天、90天、180天、365天
* 如果此任务已经运行，以下规则适用(N):
    * 请勿启动新实例
    * 并行运行新实例
    * 对新实例排队
    * 停止现有实例
