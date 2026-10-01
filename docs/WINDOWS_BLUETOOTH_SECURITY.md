# Windows 安全状态复用与配对时序调查

调查日期：2026-10-01。当前确认的是加密阶段失败；尚未确认产生失配的根因，
也没有通过连续重连验证的持久修复。本次调查没有取消配对、重置适配器或刷写固件。

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
当前应把“Windows 使用原有旧 LTK”降为候选解释，重点检查首次恢复后新产生的安全状态。

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
   已测试的注册表日志开关没有补全字段；需要验证 BTVS Full Packet Logging，必要时采用外部抓包。

| 下一轮观察 | 对原因的影响 | 可能的处理方向 |
| --- | --- | --- |
| 独立终结点配对成功，之后连续重连成功 | 增强应用连接/配对路径互相影响的假设 | 将配对和 GATT 建链分为串行阶段，仍需重复验证 |
| 明确协商不绑定，下一次却复用同一临时密钥 | 增强临时安全状态跨连接复用的假设 | 调查 Windows 兼容性；不能只靠 IsPaired 决定恢复策略 |
| 明确绑定及分发成功，重连仍找不到对应密钥 | 增强双方保存/查找不一致的假设 | 比较不同主机、适配器，以及控制器睡眠/断电前后 |
| BTHUSB 35 明确指向目标或另一 Gear VR | 转向 IRK 身份冲突 | 通过受支持 API 处理确认冲突的旧设备关系 |
| 首次成功后改变的只有适配器运行状态，重连行为也改变 | 增强 Windows/驱动运行状态假设 | 保留前后事件，进一步区分缓存、安全队列和驱动实现 |

目前不应把持续自动取消配对、盲目增加重试、降低系统安全策略或刷固件作为默认修复。
持久修复的验收条件是新进程连续重连成功，再通过控制器睡眠和断电后的重连测试。
