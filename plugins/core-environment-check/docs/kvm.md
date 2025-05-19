KVM

KVM, Kernel-based Virtual Machine, is a hypervisor built into the Linux kernel. It is similar to Xen in purpose but much simpler to get running. Unlike native QEMU, which uses emulation, KVM is a special operating mode of QEMU that uses CPU extensions (HVM) for virtualization via a kernel module.

Using KVM, one can run multiple virtual machines running unmodified GNU/Linux, Windows, or any other operating system. (See Guest Support Status for more information.) Each virtual machine has private virtualized hardware: a network card, disk, graphics card, etc.

Differences between KVM and Xen, VMware, or QEMU can be found at the KVM FAQ.

This article does not cover features common to multiple emulators using KVM as a backend. You should see related articles for such information.

Checking support for KVM
Hardware support
KVM requires that the virtual machine host's processor has virtualization support (named VT-x for Intel processors and AMD-V for AMD processors). You can check whether your processor supports hardware virtualization with the following command:

$ LC_ALL=C.UTF-8 lscpu | grep Virtualization
Alternatively:

$ grep -E --color=auto 'vmx|svm|0xc0f' /proc/cpuinfo
If nothing is displayed after running either command, then your processor does not support hardware virtualization, and you will not be able to use KVM.

Note: You may need to enable virtualization support in your BIOS. All x86_64 processors manufactured by AMD and Intel in the last 10 years support virtualization. If it looks like your processor does not support virtualization, it is almost certainly turned off in the BIOS.
Kernel support
Arch Linux kernels provide the required kernel modules to support KVM.

One can check if the necessary modules, kvm and either kvm_amd or kvm_intel, are available in the kernel with the following command:
$ zgrep CONFIG_KVM= /proc/config.gz
The module is available only if it is set to either y or m.

Then, ensure that the kernel modules are automatically loaded, with the command:
$ lsmod | grep kvm
kvm_intel             245760  0
kvmgt                  28672  0
mdev                   20480  2 kvmgt,vfio_mdev
vfio                   32768  3 kvmgt,vfio_mdev,vfio_iommu_type1
kvm                   737280  2 kvmgt,kvm_intel
irqbypass              16384  1 kvm
If the command returns nothing, the module needs to be loaded manually; see Kernel modules#Manual module handling.

Tip: If modprobing kvm_intel or kvm_amd fails but modprobing kvm succeeds, and lscpu claims that hardware acceleration is supported, check the BIOS settings. Some vendors, especially laptop vendors, disable these processor extensions by default. To determine whether there is no hardware support or whether the extensions are disabled in BIOS, the output from dmesg after having failed to modprobe will tell.
Para-virtualization with Virtio
Para-virtualization provides a fast and efficient means of communication for guests to use devices on the host machine. KVM provides para-virtualized devices to virtual machines using the Virtio API as a layer between the hypervisor and guest.

All Virtio devices have two parts: the host device and the guest driver.

Kernel support
Use the following command inside the virtual machine to check if the VIRTIO modules are available in the kernel:

$ zgrep VIRTIO /proc/config.gz
Then, check if the kernel modules are automatically loaded with the command:

$ lsmod | grep virtio
In case the above commands return nothing, you need to load the kernel modules manually.

List of para-virtualized devices
network device (virtio-net)
block device (virtio-blk)
controller device (virtio-scsi)
serial device (virtio-serial)
balloon device (virtio-balloon)
How to use KVM
See the main article: QEMU.

Tips and tricks
Note: See QEMU#Tips and tricks and QEMU/Troubleshooting for general tips and tricks.
Nested virtualization
Nested virtualization enables existing virtual machines to be run on third-party hypervisors and on other clouds without any modifications to the original virtual machines or their networking.

On host, enable nested feature for kvm_intel:

Note: the same can be done for AMD, just replace intel with amd where necessary
# modprobe -r kvm_intel
# modprobe kvm_intel nested=1
To make it permanent (see Kernel modules#Setting module options):

/etc/modprobe.d/kvm_intel.conf
options kvm_intel nested=1
Verify that feature is activated:

$ cat /sys/module/kvm_intel/parameters/nested
Y
Enable the "host passthrough" mode to forward all CPU features to the guest system:

If using QEMU, run the guest virtual machine with the following command: qemu-system-x86_64 -enable-kvm -cpu host.
If using virt-manager, change the CPU model to host-passthrough.
If using virsh, use virsh edit vm-name and change the CPU line to <cpu mode='host-passthrough' check='partial'/>
Boot the virtual machine and check if the vmx flag is present:

$ grep -E --color=auto 'vmx|svm' /proc/cpuinfo
Enabling huge pages
This article or section is a candidate for merging with QEMU.

Notes: qemu-kvm no longer exists. After the above issue is cleared, I suggest merging this section into QEMU. (Discuss in Talk:KVM)
You may also want to enable hugepages to improve the performance of your virtual machine. With an up to date Arch Linux and a running KVM, you probably already have everything you need. Check if you have the directory /dev/hugepages. If not, create it. Now we need the right permissions to use this directory. The default permission is root's uid and gid with 0755, but we want anyone in the kvm group to have access to hugepages.

Add to your /etc/fstab:

/etc/fstab
hugetlbfs       /dev/hugepages  hugetlbfs       mode=01770,gid=kvm        0 0
Instead of specifying the group name directly, with gid=kvm, you can of course specify the gid as a number, but it must match the kvm group. The mode of 1770 allows anyone in the group to create files but not unlink or rename each other's files. Make sure /dev/hugepages is mounted properly:

# umount /dev/hugepages
# mount /dev/hugepages
$ mount | grep huge
hugetlbfs on /dev/hugepages type hugetlbfs (rw,relatime,mode=1770,gid=78)
Now you can calculate how many hugepages you need. Check how large your hugepages are:

$ grep Hugepagesize /proc/meminfo
Normally that should be 2048 kB ≙ 2 MB. Let us say you want to run your virtual machine with 1024 MB. 1024 / 2 = 512. Add a few extra so we can round this up to 550. Now tell your machine how many hugepages you want:

# sysctl -w vm.nr_hugepages=550
If you had enough free memory, you should see:

$ grep HugePages_Total /proc/meminfo
HugesPages_Total:  550
If the number is smaller, close some applications or start your virtual machine with less memory (number_of_pages x 2):

$ qemu-system-x86_64 -enable-kvm -m 1024 -mem-path /dev/hugepages -hda <disk_image> [...]
Note the -mem-path parameter. This will make use of the hugepages.

Now you can check, while your virtual machine is running, how many pages are used:

$ grep HugePages /proc/meminfo
HugePages_Total:     550
HugePages_Free:       48
HugePages_Rsvd:        6
HugePages_Surp:        0
Now that everything seems to work, you can enable hugepages by default if you like. Add to your /etc/sysctl.d/40-hugepage.conf:

/etc/sysctl.d/40-hugepage.conf
vm.nr_hugepages = 550
See also:

Summary of hugetlbpage support in the Linux kernel
Debian Wiki - Hugepages
Secure Boot
This article or section is a candidate for merging with QEMU#Enabling Secure Boot.

Notes: This is not KVM-specific and would be a great addition to what is already described there. (Discuss in Talk:KVM)
KVM Secure boot has a few requirements before it can be enabled:

You must use a UEFI with secure boot support compiled in.
The UEFI must have keys enrolled.
Note: Arch Linux does not currently have a secure boot key unlike distributions like Fedora. If you intend to secure boot Arch Linux you must create your own signing key and sign your kernel after following the steps below. See Unified Extensible Firmware Interface/Secure Boot for more information.
To enable UEFI with secure boot support, install edk2-ovmf and set your virtual machine to use the secure boot enabled UEFI. If you are using libvirt, you can do this by adding the following to the XML configuration of your virtual machine.

<os firmware="efi">
  <loader readonly="yes" secure="yes" type="pflash">/usr/share/edk2/x64/OVMF_CODE.secboot.4m.fd</loader>
</os>
Next you need to enroll some keys. In this example we will enroll Microsoft and Redhat's secure boot keys. Install virt-firmware and run the following. Replace vm_name with the name of your virtual machine.

$ virt-fw-vars --input /usr/share/edk2/x64/OVMF_VARS.4m.fd --output /var/lib/libvirt/qemu/nvram/vm_name_SECURE_VARS.fd --secure-boot --enroll-redhat
Then edit the libvirt XML configuration of your virtual machine to point to the new VARS file.

<os firmware="efi">
  <loader readonly="yes" secure="yes" type="pflash">/usr/share/edk2/x64/OVMF_CODE.secboot.4m.fd</loader>
  <nvram template="/usr/share/edk2/x64/OVMF_VARS.4m.fd">/var/lib/libvirt/qemu/nvram/{vm-name}_SECURE_VARS.fd</nvram>
</os>
After this secure boot should automatically be enabled. You can double check by entering the virtual machine's BIOS by pressing F2 when you see the UEFI boot logo.

