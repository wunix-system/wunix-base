use libwunix::t;

pub fn pci_class_name(class_code: u8, subclass: u8) -> (&'static str, u16) {
    let full_class = ((class_code as u16) << 8) | (subclass as u16);
    let name = match (class_code, subclass) {
        (0x01, 0x00) => t("SCSI контроллер хранилища", "SCSI storage controller"),
        (0x01, 0x01) => t("IDE интерфейс", "IDE interface"),
        (0x01, 0x06) => t("SATA контроллер", "SATA controller"),
        (0x01, 0x08) => t("Энергонезависимая память (NVMe)", "Non-Volatile memory controller (NVMe)"),
        (0x01, _) => t("Контроллер хранилища", "Mass storage controller"),

        (0x02, 0x00) => t("Сетевой контроллер Ethernet", "Ethernet controller"),
        (0x02, 0x80) => t("Сетевой контроллер Wi-Fi", "Wireless controller"),
        (0x02, _) => t("Сетевой контроллер", "Network controller"),

        (0x03, 0x00) => t("VGA-совместимый видеоконтроллер", "VGA compatible controller"),
        (0x03, 0x02) => t("XGA видеоконтроллер", "XGA compatible controller"),
        (0x03, 0x80) => t("3D видеоконтроллер", "3D controller"),
        (0x03, _) => t("Видеоконтроллер", "Display controller"),

        (0x04, _) => t("Мультимедиа аудиоконтроллер", "Multimedia audio controller"),
        (0x05, _) => t("Контроллер памяти", "Memory controller"),

        (0x06, 0x00) => t("Хост-мост", "Host bridge"),
        (0x06, 0x01) => t("ISA-мост", "ISA bridge"),
        (0x06, 0x04) => t("PCI-to-PCI мост", "PCI bridge"),
        (0x06, _) => t("Мост шины", "Bridge device"),

        (0x07, 0x00) => t("Последовательный порт / UART", "Serial controller"),
        (0x07, _) => t("Коммуникационный контроллер", "Communication controller"),

        (0x08, _) => t("Системное периферийное устройство", "System peripheral"),
        (0x09, 0x00) => t("Контроллер клавиатуры", "Keyboard controller"),
        (0x09, 0x02) => t("Контроллер мыши", "Mouse controller"),
        (0x09, _) => t("Контроллер ввода", "Input device controller"),

        (0x0C, 0x03) => t("USB контроллер", "USB controller"),
        (0x0C, _) => t("Контроллер последовательной шины", "Serial bus controller"),

        _ => t("Периферийный контроллер", "Generic peripheral controller"),
    };
    (name, full_class)
}

pub fn pci_vendor_name(vendor: u16) -> &'static str {
    match vendor {
        0x8086 => "Intel Corporation",
        0x1AF4 => "Red Hat, Inc.",
        0x80EE => "InnoTek / Oracle (VirtualBox)",
        0x15AD => "VMware, Inc.",
        0x1414 => "Microsoft Corporation",
        0x1234 => "Bochs / QEMU",
        0x1013 => "Cirrus Logic",
        0x10EC => "Realtek Semiconductor Co., Ltd.",
        0x1B36 => "Red Hat, Inc. / QEMU",
        0x1002 => "Advanced Micro Devices, Inc. [AMD/ATI]",
        0x10DE => "NVIDIA Corporation",
        _ => t("Неизвестный производитель", "Unknown vendor"),
    }
}

pub fn pci_device_desc(vendor: u16, device: u16) -> &'static str {
    match (vendor, device) {
        (0x8086, 0x1237) => "440FX - 82441FX PMC [Natoma]",
        (0x8086, 0x7000) => "82371SB PIIX3 ISA [Natoma/Triton II]",
        (0x8086, 0x7010) => "82371SB PIIX3 IDE [Natoma/Triton II]",
        (0x8086, 0x7113) => "82371AB/EB/MB PIIX4 ACPI",
        (0x8086, 0x100E) => "82540EM Gigabit Ethernet Controller",
        (0x8086, 0x29C0) => "82G33/G31/P35/P31 Express DRAM Controller",
        (0x80EE, 0xCAFE) => "VirtualBox Guest Service (VMMDev)",
        (0x80EE, 0xBEEF) => "VirtualBox Graphics Adapter (VBoxSVGA)",
        (0x15AD, 0x0405) => "VMware SVGA II Adapter",
        (0x15AD, 0x07B0) => "VMware vmxnet3 Ethernet Controller",
        (0x15AD, 0x07C0) => "VMware PVSCSI Storage Controller",
        (0x1414, 0x5353) => "Microsoft Hyper-V Synthetic Video",
        (0x1AF4, 0x1000) | (0x1AF4, 0x1041) => "Virtio network device",
        (0x1AF4, 0x1001) | (0x1AF4, 0x1042) => "Virtio block device",
        (0x1AF4, 0x1002) | (0x1AF4, 0x1043) | (0x1AF4, 0x1045) => "Virtio memory balloon",
        (0x1AF4, 0x1003) | (0x1AF4, 0x1044) => "Virtio console",
        (0x1AF4, 0x1004) | (0x1AF4, 0x1048) => "Virtio SCSI host",
        (0x1AF4, 0x1005) => "Virtio RNG (Entropy Device)",
        (0x1AF4, 0x1050) => "Virtio GPU",
        (0x1AF4, 0x1052) => "Virtio input device",
        (0x1234, 0x1111) => "VESA/VGA Display Adapter",
        (0x1013, 0x00B8) => "GD 5446 VisualMedia",
        (0x10EC, 0x8139) => "RTL-8139/8139C/8139C+ Fast Ethernet",
        (0x10EC, 0x8168) | (0x10EC, 0x8111) => "RTL8168/8111 PCIe Gigabit Ethernet",
        (0x1B36, 0x000D) => "QEMU USB 3.0 xHCI Controller",
        (0x1B36, 0x0010) => "NVM Express Controller",
        (0x8086, 0x2668) => "82801G (ICH7 Family) High Definition Audio",
        (0x8086, 0x2415) => "82801AA AC'97 Audio",
        _ => t("Устройство PCI", "PCI Device"),
    }
}
