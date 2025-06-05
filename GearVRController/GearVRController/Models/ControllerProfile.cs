using System;

namespace GearVRController.Models
{
    public interface IControllerProfile
    {
        Guid ControllerServiceUuid { get; }
        Guid ControllerSetupCharacteristicUuid { get; }
        Guid ControllerDataCharacteristicUuid { get; }

        byte[] CmdInit1 { get; }
        byte[] CmdInit2 { get; }
        byte[] CmdInit3 { get; }
        byte[] CmdInit4 { get; }
        byte[] CmdOptimizeConnection { get; }

        int ExpectedPacketLength { get; }
        int TouchpadXOffset { get; }
        int TouchpadYOffset { get; }
        int ButtonStateOffset { get; }
        int AccelXOffset { get; }
        int AccelYOffset { get; }
        int AccelZOffset { get; }
        int GyroXOffset { get; }
        int GyroYOffset { get; }
        int GyroZOffset { get; }

        byte TouchpadButtonMask { get; }
        byte HomeButtonMask { get; }
        byte TriggerButtonMask { get; }
        byte BackButtonMask { get; }
        byte VolumeUpButtonMask { get; }
        byte VolumeDownButtonMask { get; }

        int CommandDelayMs { get; }
    }

    public class GearVRControllerProfile : IControllerProfile
    {
        public Guid ControllerServiceUuid => new Guid("4f63756c-7573-2054-6872-65656d6f7465");
        public Guid ControllerSetupCharacteristicUuid => new Guid("c8c51726-81bc-483b-a052-f7a14ea3d282");
        public Guid ControllerDataCharacteristicUuid => new Guid("c8c51726-81bc-483b-a052-f7a14ea3d281");

        public byte[] CmdInit1 => new byte[] { 0x01, 0x00 };
        public byte[] CmdInit2 => new byte[] { 0x06, 0x00 };
        public byte[] CmdInit3 => new byte[] { 0x07, 0x00 };
        public byte[] CmdInit4 => new byte[] { 0x08, 0x00 };
        public byte[] CmdOptimizeConnection => new byte[] { 0x0A, 0x02 };

        public int ExpectedPacketLength => 60;
        public int TouchpadXOffset => 54;
        public int TouchpadYOffset => 56;
        public int ButtonStateOffset => 2;
        public int AccelXOffset => 6;
        public int AccelYOffset => 8;
        public int AccelZOffset => 10;
        public int GyroXOffset => 12;
        public int GyroYOffset => 14;
        public int GyroZOffset => 16;

        public byte TouchpadButtonMask => 0b00000001;
        public byte HomeButtonMask => 0b00000010;
        public byte TriggerButtonMask => 0b00000100;
        public byte BackButtonMask => 0b00001000;
        public byte VolumeUpButtonMask => 0b00010000;
        public byte VolumeDownButtonMask => 0b00100000;

        public int CommandDelayMs => 50;
    }
}