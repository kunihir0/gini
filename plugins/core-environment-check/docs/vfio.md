PCI passthrough via OVMF

he Open Virtual Machine Firmware (OVMF) is a project to enable UEFI support for virtual machines. Starting with Linux 3.9 and recent versions of QEMU, it is now possible to passthrough a graphics card, offering the virtual machine native graphics performance which is useful for graphic-intensive tasks.

Provided you have a desktop computer with a spare GPU you can dedicate to the host (be it an integrated GPU or an old OEM card, the brands do not even need to match) and that your hardware supports it (see #Prerequisites), it is possible to have a virtual machine of any OS with its own dedicated GPU and near-native performance. For more information on techniques see the background presentation (pdf).

he Open Virtual Machine Firmware (OVMF) is a project to enable UEFI support for virtual machines. Starting with Linux 3.9 and recent versions of QEMU, it is now possible to passthrough a graphics card, offering the virtual machine native graphics performance which is useful for graphic-intensive tasks.

Provided you have a desktop computer with a spare GPU you can dedicate to the host (be it an integrated GPU or an old OEM card, the brands do not even need to match) and that your hardware supports it (see #Prerequisites), it is possible to have a virtual machine of any OS with its own dedicated GPU and near-native performance. For more information on techniques see the background presentation (pdf).