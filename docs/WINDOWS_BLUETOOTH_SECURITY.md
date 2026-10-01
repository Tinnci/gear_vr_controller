# Windows 安全状态复用与配对时序调查

调查日期：2026-10-01。当前确认的是加密阶段失败；尚未确认产生失配的根因，
也没有通过连续重连验证的持久修复。13:44 的目标取消配对请求返回 `AlreadyUnpaired`，
没有报告新删除了一条关联；本轮没有重置适配器或刷写固件。

## Full Packet Logging：确认重复使用同一组安全输入

13:41 开始的 BTVS/Wireshark 采集已经补全 HCI 加密命令参数和 Windows 发出的
SMP Pairing Request。已复核停止并保存后的完整文件，覆盖 13:42:05.931 至 13:51:29.249，
包含四次目标建链及其失败断开，解析器完整读取文件，没有报告文件截断。

| 阶段（UTC+08:00） | 应用或关联结果 | 已捕获的目标 HCI 结果 |
| --- | --- | --- |
| 13:43:38，复现当前失败 | 读取服务返回 `Unreachable` | 句柄 `0x000a` 建链成功，加密状态 `0x06`，随后本机断开 |
| 13:44:18，取消配对 | API 返回 `AlreadyUnpaired (1)` | 不能据此宣布底层安全信息已清除 |
| 13:44:36，用户确认 RGB 配对模式后，独立 PairTool 配对 | 工具退出码 0，但没有可验证的新持久关联；新建终结点仍为 `IsPaired=false`、`CanPair=false`、`Encryption` | 句柄 `0x000b` 建链成功，发出 Pairing Request，加密状态 `0x06` |
| 13:45:13，配对尝试后的首次数据连接 | 读取服务返回 `Unreachable`，未收到有效输入数据 | 句柄 `0x000c` 建链成功，加密状态 `0x06` |
| 13:45:51，新进程再次连接 | 同样返回 `Unreachable` | 句柄 `0x000d` 建链成功，加密状态 `0x06`；这是失败后的再次尝试，不能称为成功重连 |

四条目标链路的 LE Enable Encryption 命令均保留完整的 28 字节参数。
每条命令有明确的目标连接句柄，不再依赖相邻事件推测归属。本地内存比较确认：

- 四次提交的 16 字节密钥相同，Rand 相同，EDIV 相同。
- 四次的 Rand 和 EDIV 均非零。
- HCI Command Status 均为成功受理，后续 Encryption Change 均返回 `0x06`。

[Security Manager 规范 2.4.4.1、2.4.4.2](https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core_v6.3/out/en/host/security-manager-specification.html)
规定 Legacy 首次配对的 STK 加密使用零 Rand、零 EDIV；恢复 Legacy LTK 加密使用先前
分发的安全信息，Secure Connections 的 Rand、EDIV 也为零。因此本轮非零输入支持
**既有 Legacy LTK 安全信息被重复提交**，不能解释为当前配对新生成的 STK。
这是协议字段支持的推断；后续只读检查已确认持久条目也保存同一组安全输入（见下节）。
尚未证明历史命令具体从哪个存储层取值，也不知道它是否来自 10:52 的首次成功通信。
没有控制器侧记录，不能断言固件从未保存 LTK。

独立 PairTool 链路的完整时序为：

| 相对建链时间 | 事件 |
| --- | --- |
| 0 ms | LE Connection Complete 成功，目标句柄 `0x000b` |
| 11 ms | LE Enable Encryption，提交与基线相同的密钥、Rand、EDIV |
| 14 ms | Command Status：成功受理加密命令 |
| 65 ms | Windows 发出完整 SMP Pairing Request |
| 127 ms | Encryption Change：`0x06`，随后本机请求断开 |
| 141 ms | Disconnect Complete：本机断开完成 |

Pairing Request 的 IO Capability 为 `KeyboardDisplay (0x04)`，OOB 标志为零，AuthReq 为 `0x2d`：
Bonding、MITM、SC、CT2 置位；最大密钥长度 16，发起方分发掩码 `0x0e`，响应方 `0x0f`。
这些字段描述 Windows 的请求，不等于控制器已经接受对应能力或完成绑定。
完整捕获没有 Pairing Response、Confirm/Random、密钥分发或 SMP Pairing Failed。
加密请求先于新配对请求，两条过程在断开前均未完成；尚不能单凭这个顺序宣布
具体的 Windows 并发缺陷，或认定控制器因 SC/MITM 标志拒绝配对。

当前已能把恢复问题收窄到：**应用关联显示未配对、取消配对报告已经未配对，
Windows 却仍提交同一组失败的 Legacy 安全信息，独立配对也未摆脱这个状态。**
因此不能把 `IsPaired=false` 或 `AlreadyUnpaired` 当作安全状态已经清空的证据。
下一项有效对照是在完整保存本轮现场后，单独比较适配器重启前后的首次加密输入和完整 SMP。
需要捕获一次真正成功的新配对及后续重连，才能区分状态清理失效和双方密钥保存/查找失配。

原始包和密钥比较脚本只保存在本地忽略目录；报告仅保存相同与否、零值与否及状态码，
不导出密钥原文或密钥哈希。

## 只读密钥库对照：持久条目与未关联状态并存

多语言调查期间，仅访问已确认的本地适配器/目标设备路径：
`BTHPORT\Parameters\Keys\<adapter>\<target>`。没有枚举或读取其他设备密钥。
目标条目包含 `LTK`（REG_BINARY，16 字节）、`ERand`（REG_QWORD）、`EDIV`（REG_DWORD），
以及 IRK、地址、签名计数等字段名称。只读取前三项值进行本地内存比较，没有导出其原文或哈希。

四条目标 HCI 命令与该条目的 LTK 原始字节、ERand 小端整数、EDIV 均相同。
与此同时，新建 AssociationEndpoint 仍报告 `IsPaired=false`、`CanPair=false`、`Encryption`。
条目的最后写入时间为 10:52:06.544（UTC+08:00），与首次恢复时 BTHUSB 事件 8
的 10:52:06.547 相邻。该时间描述整个条目的最后写入，不证明每个值的生成或创建时间；
它支持该持久状态在首次恢复阶段被写入，而非仅凭当前结果断言最初删除前的条目从未改变。
因此现在确认的是**持久密钥信息存在且与失败命令一致，但关联 API 不报告已配对**。
这比“可能只有运行时缓存”更具体；不能据此断言 Windows 在每次命令前实际读取了注册表，
也不能在缺少成功新绑定过程时确定是哪一轮写入了这些信息。

本次检查没有改变注册表、PnP、服务或适配器状态。候选恢复的验证必须同时观察关联、
目标密钥条目及下一次加密策略。不能仅凭重启命令完成或 PnP 节点消失宣布恢复。
公开案例和候选方案见 [多语言检索与恢复方案](WINDOWS_BLUETOOTH_WORKAROUNDS.md)。

## 新发现：删除记录后又发生了配对

同一目标的 System 日志与现场采集相互对照：

| 本地时间（UTC+08:00）或阶段 | 实际证据 | 判断边界 |
| --- | --- | --- |
| 10:51:57.055 | BTHUSB 事件 10：设备不再配对，链接密钥已删除；取消配对 API 返回 `Unpaired` | Windows 报告完成了这次删除；不能证明所有驱动内存状态均已清除 |
| 10:52:06.547 | BTHUSB 事件 8：目标成功与本地适配器配对 | 首次恢复通信期间发生了新的安全配对事件；不能单独证明长期绑定或固件保存成功 |
| 首次直接 GATT 通信 | 收到 645 个有效包；API 报告 `IsPaired=false`、`CanPair=true`、`ProtectionLevel=None` | 不显式调用 PairAsync，也可能由系统在通信期间处理安全过程 |
| 正常关闭后，新进程重连 | `IsPaired=false`、`CanPair=false`、`ProtectionLevel=Encryption`；目标 Encryption Change 返回 `0x06` | 新进程没有消除故障；应用关联读数与实际安全状态需要分别观察 |
| 新建 AssociationEndpoint 查询 | 同样的三个配对字段 | 不是原 BluetoothLEDevice 对象独有的过时读数 |
| 13:02，PairTool 枚举持久关联 | 没有 Gear VR 终结点 | 另一条系统查询路径也未列出它；不能证明底层没有密钥或临时安全状态 |

原始事件和终结点枚举只保存在本地忽略的诊断目录，未提交其他设备信息或密钥。
PnP 中的服务节点仍存在；节点 `Status=OK` 只说明设备节点状态正常，不证明当前链路能加密。
适配器另有 7 条 BTHUSB 事件 18，描述无法在本地适配器保存链接密钥及 BIOS 键盘影响。
这些事件没有目标地址，也不与本次重连失败时间对应，不能用它们证明目标 LTK 在 Windows 存储失败。

“配对事件”“应用关联”“链路加密”“跨连接保存绑定”是不同证据。
在完整包尚未取得时，“Windows 使用原有旧 LTK”只能作为候选解释。
上述 Full Packet Logging 现在确认本轮重复使用同一组安全输入；其最初来源仍需捕获成功配对核对。

## 配对时序：哪些是事实，哪些仍是假设

用户重新进入 RGB 配对模式后，12:39 的目标句柄出现：

| 相对建链时间 | 事件 |
| --- | --- |
| 0 ms | LE Connection Complete 成功 |
| 11 ms | LE Enable Encryption 命令头部 |
| 89 ms | SMP Pairing Request 头部，目标句柄明确 |
| 107 ms | Encryption Change `0x06`，加密未开启 |
| 107 ms | 本机请求断开 |
| 121 ms | Disconnect Complete，本机断开完成 |

加密命令未保留句柄和密钥参数，与目标的关联依赖相邻时序；
SMP 声明长度为 7 字节，实际只记录 opcode。没有完整 Pairing Response、
AuthReq、密钥分发或成功绑定过程，因此不能宣布发现确定的并发缺陷。

[Bluetooth Security Manager 规范的 2.4.4、2.4.6 节](https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core_v6.3/out/en/host/security-manager-specification.html)
说明：拥有满足要求的安全信息时，中心设备可以直接恢复加密；否则应发起配对。
Legacy 配对的初始加密使用 STK，后续绑定重连使用 LTK；两者都通过链路加密过程建立。
因此，仅有 LE Enable Encryption 和 `0x06`，不能区分旧 LTK、新 LTK 或临时配对密钥。
规范还分别约束进行中的安全过程；本次缺少完整报文，不能据此判定违反了哪条约束。

这里的 `0x06` 来自 **HCI Encryption Change 的状态字段**，表示 PIN or Key Missing。
不要把它与 SMP Pairing Failed 的 reason `0x06`（Encryption Key Size）混用。

## 类似问题与适用范围

### Windows 将密钥缺失隐藏为不可达

[Microsoft Q&A：外设丢失绑定密钥，应用只收到 Unreachable](https://learn.microsoft.com/en-us/answers/questions/1609887/bluetooth-le-programmatically-determine-if-a-perip)
是最接近当前失败阶段的报告。2024 年的 Microsoft 回复称，已向 Bluetooth 团队确认，
当时没有 API 将这一特定密钥错误传给应用。该案例的设备仍标为已配对，
本机却是 `IsPaired=false`，所以不能把两者视为完全相同的根因。
它支持保留 HCI/ETW 诊断路径，也说明不能把所有 `Unreachable` 自动归类为密钥缺失。

### 同型号 Gear VR：重复 IRK 被拒绝

[Microsoft Q&A：两个 Gear VR 控制器分发相同 IRK](https://learn.microsoft.com/de-de/answers/questions/4184862/bluetooth-le-ger-te-mit-gleichen-identity-resolvin)
是同型号的第一手报告：第二台控制器配对时，Windows 提示身份解析密钥已被另一已配对设备使用。
IRK 用于身份解析，LTK 用于加密，不能把这个拒绝原因直接套用到当前 `0x06`。
帖子中的手改密钥方法是个案绕过，未作为本项目恢复方案。

[另一份第一手 Windows 报告](https://www.reddit.com/r/Windows10/comments/eq1snz/)
明确将相同拒绝消息定位到 System 日志的 **BTHUSB 事件 35**。
本机从 2026-09-27 起的该事件查询为零，持久关联列表也没有 Gear VR。
因此当前未找到 IRK 冲突证据；这不是对所有历史状态的排除。

Bluetooth-Policy/Operational 在本机未启用，也没有历史事件。
本机 provider 元数据列出了连接、配对尝试和密钥长度策略事件，但没有重复 IRK 事件描述。
检查该通道可以补充策略证据，不能替代对 BTHUSB 事件 35 的查询。

### 非绑定配对后错误复用安全信息

[Nordic 的非绑定配对调查](https://devzone.nordicsemi.com/f/nordic-q-a/43338/android-pairing-without-bonding---nrf-connect)
记录了另一种机制：外设回复不绑定，Android 下次连接仍尝试复用先前密钥，收到 Key Missing。
调查者比较了完整 HCI 和空中报文。该案例属于 Android 和 Nordic 外设，
只能为“临时配对状态被跨连接复用”提供机制参照，不能证明 Windows/Gear VR 存在相同缺陷。
本机的配对成功事件、未关联读数和重连加密失败使这条假设值得测试；仍需双方 bonding 标志。

### Windows 的兼容配对建议已测试失败

[Microsoft Bluetooth Developer FAQ](https://learn.microsoft.com/en-us/windows/apps/develop/devices-sensors/bluetooth-dev-faq)
针对断开后失去配对信息的旧设备，给出 `ConfirmOnly + None` 兼容配对示例。
本机三轮采集均返回 `Failed (19)`，没有 PairingRequested 回调，不能把这条建议当作现成修复。
[`PairAsync` 文档](https://learn.microsoft.com/en-us/uwp/api/windows.devices.enumeration.deviceinformationpairing.pairasync)
说明保护级别参数是最低要求；`None` 不是关闭加密或禁止绑定的可靠开关。

[Microsoft 示例仓库的另一份报告](https://github.com/microsoft/Windows-universal-samples/issues/1268)
也出现 `Failed` 和 `ProtectionLevelUsed=None`，涉及多个 Windows 10 芯片组。
页面未给出可验证的修复；失败结果里的 None 不能证明成功协商出了无加密连接。

## 工程时序检查

当前主连接路径在读取配对标记前设置 `GattSession.MaintainConnection=true`，随后未缓存发现服务。
[Microsoft GATT Client 文档](https://learn.microsoft.com/en-us/windows/apps/develop/devices-sensors/gatt-client)
说明 MaintainConnection 和未缓存发现会发起连接；单独创建 BluetoothLEDevice 并不必然发起连接。
所以，读取配对状态之前系统可能已启动连接，但不能据此认定创建对象本身引发了安全错误。

已经失败的最小查询没有 MaintainConnection；自定义配对探针也没有预先发现服务或订阅通知。
这降低了“主应用的早期 GATT 会话是唯一原因”的可能性。
后续对照仍应使用 AssociationEndpoint 直接配对，避免预先打开 BluetoothLEDevice，
以排除应用对象创建和自动连接路径的影响。

退出进程和释放会话也不等于立即完成所有 Windows 请求。
上述 GATT 文档说明连接由系统管理，未缓存请求会排队，连接过程不能直接取消。
对照实验应记录真实断开完成事件，再开始下一次操作；增加固定延迟不能单独证明安全状态已清除。

## 下一轮应验证的变量

优先捕获一次**删除后首次成功通信到下一次重连失败的完整过程**，而不是重复已有的失败请求。
全程保留 BTHPORT/BTHUSB ETW、目标 System 事件以及阶段时间线；增加 Bluetooth-Policy
的短时 ETW provider，可观察策略而不永久启用系统日志。

1. 关闭所有目标设备会话，确认断开完成。枚举并核实同一 Public 地址的 AssociationEndpoint。
   需要重新建立设备关系时，仅通过受支持 API 删除已核实目标，分别记录 `Unpaired` 和
   `AlreadyUnpaired`；后者不能当作本次确实清除了安全状态，也不应反复执行同一操作。
2. 使用已安装的 [Microsoft PairTool](https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/pairtool-examples)
   独立配对：`/associate <已核实的终结点 ID> /just-works /protection none`。
   配对阶段不创建 GATT 会话、不查询版本、不做初始化写入；记录工具结果和是否新建关联。
   `AlreadyPaired` 或“不可关联”不能算完成新的安全协商。该工具仍属 Microsoft 预览工具。
3. 配对操作完成后，新建终结点信息对象读取关联状态，再建立一次数据连接。
   成功通信后再做一次新对象查询，避免仅用连接前创建的对象判定当前状态。
   正常关闭并确认断开后，用新进程重连。
4. 若关联 API 仍称未配对却不断复用失败状态，先保存现场，再单独比较适配器重启。
   只有重启后首次加密策略确实改变，才增加 Windows 内存安全状态复用假设的可信度；
   适配器重启也会清除其他运行状态，不能单独证明某个密钥缓存缺陷。
5. 第一次成功的完整 SMP 应记录双方 bonding 标志、密钥分发掩码、Legacy/SC 路径及配对完成状态。
   用本地比较结果判断是否跨连接复用了同一密钥，不导出密钥原文。
   已测试的注册表日志开关没有补全字段；BTVS Full Packet Logging 已补全本轮失败字段，
   尚需捕获首次成功过程，必要时采用外部抓包。

| 下一轮观察 | 对原因的影响 | 可能的处理方向 |
| --- | --- | --- |
| 独立终结点配对成功，之后连续重连成功 | 增强应用连接/配对路径互相影响的假设 | 将配对和 GATT 建链分为串行阶段，仍需重复验证 |
| 明确协商不绑定，下一次却复用同一临时密钥 | 增强临时安全状态跨连接复用的假设 | 调查 Windows 兼容性；不能只靠 IsPaired 决定恢复策略 |
| 明确绑定及分发成功，重连仍找不到对应密钥 | 增强双方保存/查找不一致的假设 | 比较不同主机、适配器，以及控制器睡眠/断电前后 |
| BTHUSB 35 明确指向目标或另一 Gear VR | 转向 IRK 身份冲突 | 通过受支持 API 处理确认冲突的旧设备关系 |
| 首次成功后改变的只有适配器运行状态，重连行为也改变 | 增强 Windows/驱动运行状态假设 | 保留前后事件，进一步区分缓存、安全队列和驱动实现 |

目前不应把持续自动取消配对、盲目增加重试、降低系统安全策略或刷固件作为默认修复。
持久修复的验收条件是新进程连续重连成功，再通过控制器睡眠和断电后的重连测试。

## Windows 当前是否已修复：2026-10-01 核对

本机实际构建为 **26220.9568、25H2**，对应 Windows 11 Beta；注册表 ProductName 的
旧名称不能作为 Windows 10 的判断依据。[该版本官方发布说明](https://learn.microsoft.com/en-us/windows-insider/release-notes/beta/preview-build-26220-9568)
没有列出本次密钥复用/配对时序问题的修复。这只说明没有找到明确的公开修复说明，
不意味着 Microsoft 从未修改相关实现。故障在这个版本上已复现，不能说当前环境已经解决。

[2026 年 6 月 KB5095093](https://support.microsoft.com/en-us/servicing/os/windows-11/2026/06/june-23-2026-kb5095093-os-builds-26200-8737-and-26100-8737-preview)
确实修复了射频不可用或适配器变化后删除设备出现 Remove failed 的问题。
其触发条件与本机删除成功、首次通信成功、再次加密失败不同，不能把它作为本次根因已修复的证据。

当前公开的 [GattCommunicationStatus](https://learn.microsoft.com/en-us/uwp/api/windows.devices.bluetooth.genericattributeprofile.gattcommunicationstatus)
仍只有 Success、Unreachable、ProtocolError、AccessDenied，没有专门的 Key Missing 状态。
结合前述 2024 年 Q&A，目前没有找到公开的新接口解决该错误细节传递问题。
API 是否能呈现错误与 Windows 是否能恢复连接，是两个不同问题。

## 官方解析器复核现有 ETL

本机 Windows SDK 10.0.26100.0 已带 x64 BTETLParse，Wireshark 已安装。
使用 [Microsoft BTETLParse](https://learn.microsoft.com/en-us/windows-hardware/drivers/bluetooth/testing-btp-tools-btetlparse)
将 12:39 和 12:47 的原始 port.etl 转换为 PCAPNG，再仅导出安全元数据。

- 12:39 的加密命令、Pairing Request 和 `0x06` 顺序与原 XML 分析一致。
- Pairing Request 在官方解析结果中标为 **Sent**，句柄分别为 `0x0013`、`0x0016`。
- 请求帧含 H4 头部共 10 字节，但 L2CAP 声明 SMP 长度 7，实际仍只有 1 字节 opcode。
  AuthReq、bonding 标志、密钥长度和分发掩码在 Wireshark 中同样没有值。
- 三轮 ETW 文件头的 EventsLost、BuffersLost 都为零，不能用采集队列溢出来解释这些缺失字段。
- Wireshark 的 Malformed 标记来自本地记录短于声明长度；不能据此认定空中报文或固件生成了坏包。

转换工具不能恢复采集时已省略的数据。原始 PCAPNG 和只含元数据的 CSV 均留在忽略目录。

## 补齐 Windows 内部决策日志

已下载 [Microsoft BluetoothStack WPR 配置及说明](https://github.com/microsoft/busiotools/blob/master/bluetooth/tracing/readme.md)，
并用本机 `wpr -profiles` 验证配置可识别；尚未启动记录。
本地配置 SHA-256 为 `D69C983384B970E8C0380C48EBEBBA547F94C4A84C9F4912A10798B6B08CF704`。

此前三轮 BTHPORT 记录只有事件 402（包数据）和 403（WDFFILEOBJECT），
并没有其他已解码的安全状态机事件。官方配置额外包含 BthWinRT、设备枚举、BTHSERV、
BthLeEnum、BthPort WPP 和遥测 provider，因此它与“再抓一次相同 HCI provider”不同。
WPP 的可解码程度取决于匹配的格式信息；内部原因可能需要 Microsoft 分析，不能承诺全部公开可读。

下一轮在管理员 PowerShell 中采用普通短时模式：

```powershell
$traceRoot = 'C:\Users\shiso\Downloads\gear_vr_controller\dist\diagnostics\2026-10-01-recovery'
$traceProfile = Join-Path $traceRoot 'BluetoothStack.wprp'
wpr.exe -status
# 确认没有其他 WPR 记录后启动；本次未执行这条命令。
wpr.exe -start ($traceProfile + '!BluetoothStack') -filemode
# 在此完成单轮独立配对、首次连接和新进程重连，记录每阶段的 UTC 时间。
wpr.exe -stop (Join-Path $traceRoot 'BthTracing.etl')
```

先捕获未重置时的故障，再单独比较适配器重启。不能在保存现场前按通用说明先切换射频，
否则会丢失要调查的安全状态。File 模式没有这里原先 64 MB 环形文件的同等上限，
应限制为完成一轮实验所需的短时间，并在异常路径也停止本次记录。

早期调查需要 [BTVS Full Packet Logging](https://learn.microsoft.com/en-us/windows-hardware/drivers/bluetooth/testing-btp-tools-btvs)
补全协议字段；本机 13:41 开始的该工具采集已经补全本轮失败的加密参数和 Pairing Request。
文档将该功能列为图形窗口按钮，没有公开对应命令行开关。
不能虚构一个 CLI 参数，也不能把它与发送/接受调试密钥的 Debug Mode 混用。
完整日志留在本地，分析输出只包含标志、阶段、状态码和密钥是否相同等结果。

| 层次 | 需要对齐的证据 | 要回答的问题 |
| --- | --- | --- |
| 应用与 PairTool | 进程、阶段、UTC 时间、终结点 ID、API 完成结果 | 是哪个请求发起安全过程，是否有另一个连接请求重叠？ |
| Windows WPR | WinRT、关联、驱动 WPP、策略和遥测事件 | 系统为何认为已有可用安全信息；哪一步失败后触发断开？ |
| HCI/SMP | 地址到句柄映射、方向、完整 Pairing Request/Response、分发、Encryption Change | 实际协商绑定还是临时配对；重连是否复用先前安全信息？ |
| System/PnP | BTHUSB 8、10、35，目标节点与适配器版本 | 新关系何时生成、删除，是否发生身份冲突或适配器变化？ |

每条记录应先按目标地址和句柄归属，再按 UTC 时间、ActivityID（若有效）和进程/请求关联。
句柄可能在断开后复用，不能跨整个文件只按相同句柄拼接；加密命令参数缺失时必须标明时间关联。
对照至少包含失败状态、独立 PairTool 路径、适配器重启后的状态。若仍无法区分主机和外设，
用另一稳定版 Windows 主机或另一适配器控制变量复测，而不是同时更新系统、驱动和固件。
只有看到过程改变及多次重连成功，才能把具体恢复操作升级为应用方案。
