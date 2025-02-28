# Gear VR 控制器 Windows 应用程序

这是一个用于 Gear VR 控制器的 Windows 应用程序，允许用户将 Gear VR 控制器作为电脑输入设备使用。

## 功能特点

1. **蓝牙连接管理**

   - 自动搜索并连接 Gear VR 控制器
   - 实时显示连接状态
   - 支持断开连接和重新连接

2. **触摸板控制**

   - 将触摸板映射为鼠标移动
   - 支持触摸板点击作为鼠标点击
   - 智能化的触摸板校准功能
     - 自动化的多步骤校准流程
     - 实时进度显示和状态反馈
     - 智能检测和自动进入下一步
   - 可调节的鼠标灵敏度

3. **按键映射**

   - 支持触发键和返回键的自定义映射
   - 可配置的按键组合

4. **运动控制**

   - 支持陀螺仪数据读取
   - 实时显示传感器数据

5. **用户界面**
   - 现代化的 WinUI 3 界面
   - 实时状态显示
   - 直观的设置界面
   - 智能校准向导
     - 清晰的步骤指示器
     - 双进度条显示（触摸结束检测和自动进入倒计时）
     - 实时数据可视化

## 项目结构

```
GearVRController/
├── GearVRController/              # 主项目目录
│   ├── Models/                    # 数据模型
│   │   └── ControllerData.cs     # 控制器数据模型
│   ├── ViewModels/               # 视图模型
│   │   ├── MainViewModel.cs      # 主窗口视图模型
│   │   └── TouchpadCalibrationViewModel.cs  # 触摸板校准视图模型
│   ├── Views/                    # 视图
│   │   ├── TouchpadCalibrationWindow.xaml   # 触摸板校准窗口
│   │   └── TouchpadCalibrationWindow.xaml.cs # 触摸板校准窗口代码
│   ├── Services/                 # 服务
│   │   ├── BluetoothService.cs   # 蓝牙服务
│   │   ├── ControllerService.cs  # 控制器服务
│   │   ├── LocalSettingsService.cs # 本地设置服务
│   │   ├── WindowsInputSimulator.cs # Windows输入模拟器
│   │   ├── ServiceLocator.cs     # 服务定位器
│   │   └── Interfaces/           # 服务接口
│   ├── Helpers/                  # 辅助类
│   │   └── InputSimulator.cs     # 输入模拟器
│   ├── Converters/              # 值转换器
│   │   ├── BoolToControlStateConverter.cs   # 控制状态转换器
│   │   ├── BooleanConverters.cs  # 布尔值转换器
│   │   └── StatusConverters.cs   # 状态转换器
│   ├── Assets/                  # 资源文件
│   ├── Properties/              # 项目属性
│   ├── MainWindow.xaml          # 主窗口界面
│   ├── MainWindow.xaml.cs       # 主窗口代码
│   ├── App.xaml                 # 应用程序资源
│   ├── App.xaml.cs              # 应用程序代码
│   ├── Package.appxmanifest     # 应用程序清单
│   └── app.manifest             # 应用程序清单
└── GearVRController.sln         # 解决方案文件
```

## 主要功能模块

### 1. 蓝牙连接 (BluetoothService)

- 处理与 Gear VR 控制器的蓝牙通信
- 实现数据包的解析和处理
- 管理连接生命周期

### 2. 输入模拟 (InputSimulator)

- 模拟鼠标和键盘输入
- 处理触摸板移动到鼠标移动的转换
- 实现按键映射功能

### 3. 触摸板校准 (TouchpadCalibrationViewModel)

- 全自动化的多步骤校准流程
  1. 边界校准：通过圆周运动自动检测触摸板边界
  2. 中心点校准：智能判断中心点位置
  3. 方向校准：自动采集和验证四个方向的移动数据
- 智能检测功能
  - 实时验证移动数据的有效性
  - 自动过滤无效的触摸点
  - 智能判断校准完成条件
- 用户友好的反馈机制
  - 双进度条显示系统
  - 实时状态提示
  - 自动化的步骤转换

### 4. 主界面控制 (MainViewModel)

- 管理应用程序状态
- 处理用户设置
- 协调各个功能模块

## 使用说明

1. **连接控制器**

   - 启动应用程序
   - 按住控制器触发键进入配对模式
   - 点击"连接"按钮

2. **校准触摸板**

   - 点击"校准"按钮，校准窗口会自动开始校准流程
   - 按照界面提示完成以下步骤：
     1. 在触摸板边缘划圈，确定边界范围
     2. 点击触摸板中心位置
     3. 依次完成上、下、左、右四个方向的滑动校准
   - 每个步骤完成后会自动进入下一步
   - 校准完成后自动保存设置

3. **自定义设置**
   - 调节鼠标灵敏度
   - 配置按键映射
   - 启用/禁用特定功能

## 系统要求

- Windows 10 版本 1809 或更高
- 支持蓝牙 4.0 或更高版本
- WinUI 3 运行时

## 开发环境

- Visual Studio 2022
- .NET 6.0 或更高版本
- Windows App SDK 1.2 或更高版本

## 注意事项

1. 校准过程全自动化，无需手动点击"开始校准"
2. 确保控制器电量充足
3. 保持控制器在蓝牙有效范围内
4. 如遇连接问题，请尝试重启控制器

## 未来计划

1. 添加更多自定义选项
2. 优化触摸板响应
3. 支持手势识别
4. 添加配置文件导入/导出功能
5. 进一步优化校准算法
6. 添加校准数据可视化功能
