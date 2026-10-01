# Gear VR 连接失败：现场分析与恢复实验（2026-10-01 更新）

本次失败已定位到链路加密阶段：Windows 与控制器成功建立 BLE 链路，
随后加密返回 `0x06 / PIN or Key Missing`，本机再发起断开。
应用看到的 `GATT Unreachable (1)` 是后续结果。

这次现场最可能是 Windows 保留了旧配对记录，而控制器没有提供该记录对应的
长期密钥（LTK）。目前不能确定密钥何时、为何丢失，也不能据此解释所有历史故障。
2026-10-01 已移除这一台设备的旧配对记录，首次直接连接收到有效数据，
但正常断开后的重连再次出现同一加密错误。单次连接成功不能视为稳定修复。
本次没有重启电脑、重置适配器或重启蓝牙服务。

## 2026-10-01 恢复实验

用户确认控制器已唤醒后，重新扫描验证了同一 Public 地址和设备身份。
只对这台控制器调用取消配对 API；没有删除其他设备或修改密钥注册表。

| 操作 | 实际结果 | 能说明什么 |
| --- | --- | --- |
| 移除旧设备关系 | `UnpairAsync` 返回 `Unpaired (0)` | Windows 已完成本次取消配对 |
| 新进程直接连接，采集 10 秒 | 服务读取、通知订阅、初始化写入成功；收到 645 个有效数据包，约 63.9 包/秒 | 当前协议和 GATT 路径能够工作，首次恢复不需要重启 |
| 正常退出后，新进程再次连接 | 服务读取返回 `Unreachable` | 首次成功不足以证明可重复连接 |
| 再次采集目标 HCI | 建链成功，句柄 `0x000E`；Encryption Change 原始数据 `08 04 06 0E 00 00` | 重连仍在加密阶段因 `0x06` 失败 |
| 重新创建 AssociationEndpoint 设备信息 | `IsPaired=false`、`CanPair=false`、`ProtectionLevel=Encryption`，与 BluetoothLEDevice 查询一致 | “未配对”也不能证明 Windows 没有相关安全状态；不是旧对象独有的读数 |
| 再次取消配对 | `AlreadyUnpaired (1)` | 这次没有证明失效密钥已被清除 |
| 自定义 `ConfirmOnly + None` 配对实验 | `Failed (19)`，没有触发 PairingRequested 回调 | 没有建立新设备关系，也没有验证实际保护级别已改变 |

首次数据采集还记录了一个不能解析为传感器数据的通知。协议允许两字节命令响应，
当前 CLI 将这种响应也计入 `invalid_packets`；不能据此认定传感器数据损坏。

下一步先确认控制器进入配对模式，再进行新配对和连续重连实验。
[三星的配对说明](https://www.samsung.com/hk_en/support/mobile-devices/connect-samsung-gear-vr-controller-via-bluetooth/)
要求长按主页键，直到指示灯红、绿、蓝闪烁。普通唤醒不能代替这个状态。
关闭可能连接控制器的其他主机蓝牙，有助于控制实验条件。

## 已验证的证据

- Windows 报告该控制器 `paired=true`、设备访问 `Allowed`，配对保护级别为 `Encryption`。
- 蓝牙射频开启，`bthserv`、`BthLEEnum` 运行，目标 PnP 记录存在且问题代码为零。
- 15 秒内收到目标 50 条广播，类型是可连接广播，RSSI 约 −41 至 −55 dBm。
- 广播没有 Gear VR 服务 UUID。服务过滤扫描为空，但全部设备扫描可以识别它。
  广播服务列表为空不等于设备没有对应 GATT 服务。
- 最小 Windows 查询不创建保持连接的 GATT 会话，也不发送控制器初始化命令。
  它的未缓存服务读取仍在约 7.8 秒后返回 `Unreachable`。
- 缓存读取约 33 毫秒成功，列出七个服务，包括 Gear VR 服务 UUID。
  读到本地缓存不能证明控制器当前可通信。

随后采集了 BTHPORT 和 BTHUSB 的短时 ETW。BTHPORT 中的目标地址、连接句柄和
HCI 事件共同组成下面的证据链。时间以发起连接命令为零点，四舍五入到毫秒：

| 相对时间 | HCI 事件或命令 | 结果 |
| --- | --- | --- |
| 0 ms | LE Extended Create Connection，目标为已确认的 Public 地址 | 开始建立链路 |
| 4 ms | Command Status | `0x00`，命令被接受 |
| 109 ms | LE Enhanced Connection Complete | `0x00`，链路成功，句柄 `0x000A` |
| 120 ms | LE Enable Encryption（旧称 Start Encryption） | 开始链路加密 |
| 123 ms | Command Status | `0x00`，加密命令被接受，不代表加密已成功 |
| 276 ms | Encryption Change，句柄 `0x000A` | 状态 `0x06`，加密未开启 |
| 276 ms | 本机 HCI Disconnect | 本机要求断开 |
| 297 ms | Disconnection Complete | `0x16 / Connection Terminated by Local Host` |

关键失败事件原始数据为 `08 04 06 0A 00 00`：
事件码 `0x08`，参数长度 4，状态 `0x06`，句柄 `0x000A`，加密状态 0。
HCI 命令状态、链路状态、加密状态和 GATT 状态必须分别判断。
记录中没有采集到控制器初始化或应用输入注入。

## 为什么优先处理配对记录

[Bluetooth 错误码规范](https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-54/out/en/architecture,-mixing,-and-conventions/controller-error-codes.html)
将 `0x06` 定义为配对 PIN 缺失或认证密钥缺失。
本次已经建链，失败发生在加密阶段，应优先检查密钥，而不是尝试输入通用 PIN。

[Bluetooth LE 链路层规范](https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-54/out/en/low-energy-controller/link-layer-specification.html)
规定：外设没有提供可用 LTK 时，可以用 `0x06` 拒绝加密请求，再向中心设备的主机报告。
这与当前 HCI 序列相符。它支持“双方没有可用的对应密钥”的判断，
但不能单凭这份主机追踪证明控制器固件损坏或 Qualcomm 驱动存在缺陷。

Windows 中的“已配对”反映保存的设备关系，不是一次成功的密钥验证。
本次 CLI 的 `pair` 仅返回现有配对状态；它没有执行新的配对协商。
Windows 的 [`AlreadyPaired`](https://learn.microsoft.com/en-us/uwp/api/windows.devices.enumeration.devicepairingresultstatus)
结果也不应作为密钥已更新或链路已恢复的证明。

重启可能清除挂起会话和驱动临时状态，但不会替代双方重新建立密钥。
以前“删除设备并重启”把两个操作组合在一起，不能据此认定重启不可缺少。
应逐个操作并比较结果。

## 首选恢复实验：不先重启

这一步改变 Windows 保存的设备关系，应由用户确认后执行，或者由用户在设置中操作。
只处理已通过地址确认的 Gear VR 控制器。

1. 关闭控制面板和其他可能持有该控制器的应用。所有诊断 CLI 正常退出。
2. 在 Windows 蓝牙设置中移除该 Gear VR 控制器，等待操作完成。
   程序化实现应使用 `DeviceInformation.Pairing.UnpairAsync` 并检查结果，
   而不是批量删除 PnP 节点或手改注册表。
3. 唤醒控制器并保持其可发现、可连接。重新扫描，确认 Public 地址和设备身份。
4. 先通过应用现有的直接 GATT 路径连接；如果 Windows 请求配对，完成正常确认。
   若明确需要先配对，再建立一次新配对后连接。
5. 连续观察至少 10 秒：`valid_packets` 持续增长，最近数据包年龄保持较小。
   正常断开后，使用新进程重复连接至少两次；必要时再比较休眠或断电后的行为。
   保存每次输出。只有首次成功不能通过恢复验证。

[Microsoft 配对说明](https://learn.microsoft.com/en-us/windows/uwp/devices-sensors/pair-devices)
支持由设备 API 处理必要的配对，也提供明确的取消配对 API。
[Microsoft 缓存说明](https://learn.microsoft.com/en-us/uwp/api/windows.devices.bluetooth.bluetoothcachemode)
说明 GATT 缓存由操作系统统一维护，取消配对会使相关缓存失效。
因此移除这台设备的旧配对比增加读取重试次数更符合当前证据。

发布包中的诊断工具可用于前后对照；地址请使用扫描或 Windows 记录中核实的值：

```powershell
$address = 'AABBCCDDEEFF' # 替换为实际控制器地址
$env:RUST_LOG = 'info,gear_vr_controller_rust::infrastructure::bluetooth=debug'
./gearvr-debug.exe status --address $address --address-type public
./gearvr-debug.exe scan --all --seconds 15
./gearvr-debug.exe connect --address $address --address-type public --seconds 10 --timeout 40
```

确实需要一次新的配对时才运行：

```powershell
./gearvr-debug.exe pair --address $address --address-type public
```

本次没有验证 CLI 的新配对确认流程，只验证了现有配对分支。
如果 CLI 配对返回处理器未注册等结果，可先使用 Windows 设置完成新配对，
再使用 CLI 验证 GATT 通信，避免把配对界面问题与当前密钥故障混为一谈。

## 失败或复发时怎样区分

| 结果 | 下一步 |
| --- | --- |
| 移除旧配对后直接 GATT 持续收到数据，并且断开后连续重连成功 | 维持正常设备 API 路径，不增加预先强制配对 |
| 首次连接成功，但 `IsPaired=false` 的重连仍出现 `0x06` | 比较新建 AssociationEndpoint 读数、实际加密事件和配对模式；`AlreadyUnpaired` 不能作为密钥已清除的证据 |
| 新配对后恢复，控制器休眠或断电后再次出现 `0x06` | 比较控制器是否保留绑定、是否与其他主机重新配对，以及 Windows 使用的保护级别；再采集同一故障的 HCI 序列 |
| 确认已移除并新建配对，但仍出现 `0x06` | 核对实际目标地址、取消配对结果和新配对是否真的执行成功，再调查密钥保存或固件/驱动行为 |
| 取消配对后失败原因变为链路超时、建链失败等 | 根据新的 HCI 原因分析射频、连接队列或驱动，不能沿用旧结论 |
| 设备关系无法清除或平台状态无法恢复 | 在保存现场后，分别比较服务恢复、适配器恢复、重启的效果，避免一次改变全部条件 |

Microsoft 的
[Bluetooth Developer FAQ](https://learn.microsoft.com/en-us/windows/apps/develop/devices-sensors/bluetooth-dev-faq)
专门讨论断开后无法重连的旧设备，指出部分设备会丢失配对信息，
并给出自定义 `ConfirmOnly + None` 的兼容配对示例。
它为当前测试提供了依据，但不保证适用于每个设备和驱动组合。

供兼容性实验参考：原始
[Gear VR Windows 实现](https://github.com/rdady/gear-vr-controller-windows/blob/5f4172971107244c526b4780665627690b5fcd90/gear-vr-controller.linq)
使用自定义 `ConfirmOnly` 配对，最低保护要求为 `None`。
这为“新配对时比较保护要求”提供了具体参考，不能证明本次只改这个参数就会恢复。
[Microsoft PairAsync 说明](https://learn.microsoft.com/en-us/uwp/api/windows.devices.enumeration.deviceinformationpairing.pairasync)
指出这是最低要求，实际协商可以选择更高等级。`None` 不是可靠的关闭加密开关。
当前已有配对的保护级别也不能通过一次返回 `AlreadyPaired` 的调用来确认已改变。

## 应用应该如何改进

- 继续检查未缓存服务读取、初始化写入和通知订阅的真实结果。
  不能改用缓存成功来掩盖建链失败。
- 将“打开句柄”“建链”“加密”“协议初始化”“收到有效数据”分别呈现和记录。
  当前高层 WinRT 返回 `Unreachable` 时，不能凭猜测改报 `Key Missing`。
- 对已配对但不可达的设备，可以提供重新配对的诊断入口；设备关系删除必须显式确认。
  正常连接和后台重连不能自动取消配对。
- 明确标识广播名称、历史设备和服务匹配证据。已确认的控制器缺少广播服务 UUID
  时仍应可找到，但不能把任意同名设备自动连接为可信控制器。
- 将短时 ETW 作为用户启动的深入诊断，常规日志保留阶段、耗时、状态码和关联 ID。
  长期记录全部 HCI 包不适合作为默认日志策略。

## 验证边界

以上关于当前失败阶段的结论来自实际广播、最小 GATT 查询和目标 HCI 事件。
恢复实验已经证明首次数据通信不需要重启，但稳定重连和睡眠/断电后不复发，
仍没有通过验证。自定义配对测试失败，不能宣称它已解决问题。
2026-09-27 的历史日志只有高层不可达结果，
不能证明那些失败也具有同一个 `0x06` 原因。
