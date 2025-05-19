QEMU

According to the QEMU about page:

QEMU is a generic and open source machine emulator and virtualizer.
When used as a machine emulator, QEMU can run OSes and programs made for one machine (e.g. an ARM board) on a different machine (e.g. your x86 PC). By using dynamic translation, it achieves very good performance.
QEMU can use other hypervisors like Xen or KVM to use CPU extensions (HVM) for virtualization. When used as a virtualizer, QEMU achieves near native performances by executing the guest code directly on the host CPU.

Installation
Install the qemu-full package (or qemu-base for the version without GUI and qemu-desktop for the version with only x86_64 emulation by default) and below optional packages for your needs:

qemu-block-gluster - Glusterfs block support
qemu-block-iscsi - iSCSI block support
samba - SMB/CIFS server support
Alternatively, qemu-user-static exists as a usermode and static variant.

QEMU variants
QEMU is offered in several variants suited for different use cases.

As a first classification, QEMU is offered in full-system and usermode emulation modes:

Full-system emulation
In this mode, QEMU emulates a full system, including one or several processors and various peripherals. It is more accurate but slower, and does not require the emulated OS to be Linux.
QEMU commands for full-system emulation are named qemu-system-target_architecture, e.g. qemu-system-x86_64 for emulating x86_64 CPUs, qemu-system-i386 for Intel 32-bit x86 CPUs, qemu-system-arm for ARM (32 bits), qemu-system-aarch64 for ARM64, etc.
If the target architecture matches the host CPU, this mode may still benefit from a significant speedup by using a hypervisor like KVM or Xen.
Usermode emulation
In this mode, QEMU is able to invoke a Linux executable compiled for a (potentially) different architecture by leveraging the host system resources. There may be compatibility issues, e.g. some features may not be implemented, dynamically linked executables will not work out of the box (see #Chrooting into arm/arm64 environment from x86_64 to address this) and only Linux is supported (although Wine may be used for running Windows executables).
QEMU commands for usermode emulation are named qemu-target_architecture, e.g. qemu-x86_64 for emulating 64-bit CPUs.
QEMU is offered in dynamically-linked and statically-linked variants:

Dynamically-linked (default)
qemu-* commands depend on the host OS libraries, so executables are smaller.
Statically-linked
qemu-* commands can be copied to any Linux system with the same architecture.
In the case of Arch Linux, full-system emulation is offered as:

Non-headless (default)
This variant enables GUI features that require additional dependencies (like SDL or GTK).
Headless
This is a slimmer variant that does not require GUI (this is suitable e.g. for servers).
Note that headless and non-headless versions install commands with the same name (e.g. qemu-system-x86_64) and thus cannot be both installed at the same time.

Details on packages available in Arch Linux
The qemu-desktop package provides the x86_64 architecture emulators for full-system emulation (qemu-system-x86_64). The qemu-emulators-full package provides the x86_64 usermode variant (qemu-x86_64) and also for the rest of supported architectures it includes both full-system and usermode variants (e.g. qemu-system-arm and qemu-arm).
The headless versions of these packages (only applicable to full-system emulation) are qemu-base (x86_64-only) and qemu-emulators-full (rest of architectures).
Full-system emulation can be expanded with some QEMU modules present in separate packages: qemu-block-gluster, qemu-block-iscsi and qemu-guest-agent.
qemu-user-static provides a usermode and static variant for all target architectures supported by QEMU. The installed QEMU commands are named qemu-target_architecture-static, for example, qemu-x86_64-static for intel 64-bit CPUs.
Note: At present, Arch does not offer a full-system mode and statically linked variant (neither officially nor via AUR), as this is usually not needed.
Graphical front-ends for QEMU
Unlike other virtualization programs such as VirtualBox and VMware, QEMU does not provide a GUI to manage virtual machines (other than the window that appears when running a virtual machine), nor does it provide a way to create persistent virtual machines with saved settings. All parameters to run a virtual machine must be specified on the command line at every launch, unless you have created a custom script to start your virtual machine(s).

Libvirt provides a convenient way to manage QEMU virtual machines. See list of libvirt clients for available front-ends.

Creating a new virtualized system
Creating a hard disk image
The factual accuracy of this article or section is disputed.

Reason: If I get the man page right the raw format only allocates the full size if the filesystem does not support "holes" or it is explicitly told to preallocate. See qemu-img(1) § NOTES. (Discuss in Talk:QEMU)
Tip: See Wikibooks:QEMU/Images for more information on QEMU images.
To run QEMU you will need a hard disk image, unless you are booting a live system from CD-ROM or the network (and not doing so to install an operating system to a hard disk image). A hard disk image is a file which stores the contents of the emulated hard disk.

A hard disk image can be raw, so that it is literally byte-by-byte the same as what the guest sees, and will always use the full capacity of the guest hard drive on the host. This method provides the least I/O overhead, but can waste a lot of space, as not-used space on the guest cannot be used on the host.

Alternatively, the hard disk image can be in a format such as qcow2 which only allocates space to the image file when the guest operating system actually writes to those sectors on its virtual hard disk. The image appears as the full size to the guest operating system, even though it may take up only a very small amount of space on the host system. This image format also supports QEMU snapshotting functionality (see #Creating and managing snapshots via the monitor console for details). However, using this format instead of raw will likely affect performance.

QEMU provides the qemu-img command to create hard disk images. For example to create a 4 GiB image in the raw format:

$ qemu-img create -f raw image_file 4G
You may use -f qcow2 to create a qcow2 disk instead.

Note: You can also simply create a raw image by creating a file of the needed size using dd or fallocate.
Warning: If you store the hard disk images on a Btrfs file system, you should consider disabling Copy-on-Write for the directory before creating any images. Can be specified in option nocow for qcow2 format when creating image:
$ qemu-img create -f qcow2 image_file -o nocow=on 4G
Overlay storage images
You can create a storage image once (the 'backing' image) and have QEMU keep mutations to this image in an overlay image. This allows you to revert to a previous state of this storage image. You could revert by creating a new overlay image at the time you wish to revert, based on the original backing image.

To create an overlay image, issue a command like:

$ qemu-img create -o backing_file=img1.raw,backing_fmt=raw -f qcow2 img1.cow
After that you can run your QEMU virtual machine as usual (see #Running a virtualized system):

$ qemu-system-x86_64 img1.cow
The backing image will then be left intact and mutations to this storage will be recorded in the overlay image file.

When the path to the backing image changes, repair is required.

Warning: The backing image's absolute filesystem path is stored in the (binary) overlay image file. Changing the backing image's path requires some effort.
Make sure that the original backing image's path still leads to this image. If necessary, make a symbolic link at the original path to the new path. Then issue a command like:

$ qemu-img rebase -b /new/img1.raw /new/img1.cow
At your discretion, you may alternatively perform an 'unsafe' rebase where the old path to the backing image is not checked:

$ qemu-img rebase -u -b /new/img1.raw /new/img1.cow
Resizing an image
Warning: Resizing an image containing an NTFS boot file system could make the operating system installed on it unbootable. It is recommended to create a backup first.
The qemu-img executable has the resize option, which enables easy resizing of a hard drive image. It works for both raw and qcow2. For example, to increase image space by 10 GiB, run:

$ qemu-img resize disk_image +10G
After enlarging the disk image, you must use file system and partitioning tools inside the virtual machine to actually begin using the new space.

Shrinking an image
When shrinking a disk image, you must first reduce the allocated file systems and partition sizes using the file system and partitioning tools inside the virtual machine and then shrink the disk image accordingly. For a Windows guest, this can be performed from the "create and format hard disk partitions" control panel.

Warning: Proceeding to shrink the disk image without reducing the guest partition sizes will result in data loss.
Then, to decrease image space by 10 GiB, run:

$ qemu-img resize --shrink disk_image -10G
Converting an image
You can convert an image to other formats using qemu-img convert. This example shows how to convert a raw image to qcow2:

$ qemu-img convert -f raw -O qcow2 input.img output.qcow2
This will not remove the original input file.

Preparing the installation media
To install an operating system into your disk image, you need the installation medium (e.g. optical disc, USB-drive, or ISO image) for the operating system. The installation medium should not be mounted because QEMU accesses the media directly.

Tip: If using an optical disc, it is a good idea to first dump the media to a file because this both improves performance and does not require you to have direct access to the devices (that is, you can run QEMU as a regular user without having to change access permissions on the media's device file). For example, if the CD-ROM device node is named /dev/cdrom, you can dump it to a file with the command:
$ dd if=/dev/cdrom of=cd_image.iso bs=4k
Installing the operating system
This is the first time you will need to start the emulator. To install the operating system on the disk image, you must attach both the disk image and the installation media to the virtual machine, and have it boot from the installation media.

For example on i386 guests, to install from a bootable ISO file as CD-ROM and a raw disk image:

$ qemu-system-x86_64 -cdrom iso_image -boot order=d -drive file=disk_image,format=raw
See qemu(1) for more information about loading other media types (such as floppy, disk images or physical drives) and #Running a virtualized system for other useful options.

After the operating system has finished installing, the QEMU image can be booted directly (see #Running a virtualized system).

Note: By default only 128 MiB of memory is assigned to the machine. The amount of memory can be adjusted with the -m switch, for example -m 512M or -m 2G.
Tip:
Instead of specifying -boot order=x, some users may feel more comfortable using a boot menu: -boot menu=on, at least during configuration and experimentation.
When running QEMU in headless mode, it starts a local VNC server on port 5900 per default. You can use TigerVNC to connect to the guest OS: vncviewer :5900
If you need to replace floppies or CDs as part of the installation process, you can use the QEMU machine monitor (press Ctrl+Alt+2 in the virtual machine's window) to remove and attach storage devices to a virtual machine. Type info block to see the block devices, and use the change command to swap out a device. Press Ctrl+Alt+1 to go back to the virtual machine.
Pre-made virtual machine images
In many cases, it's not necessary or desired to manually install your own operating system, for instance in a cloud environment. Luckily, many pre-made images are available for download from different providers.

For Arch Linux, we have the arch-boxes project with weekly image releases.

There are similar images available for Fedora and Debian.

Running a virtualized system
qemu-system-* binaries (for example qemu-system-i386 or qemu-system-x86_64, depending on guest's architecture) are used to run the virtualized guest. The usage is:

$ qemu-system-x86_64 options disk_image
Options are the same for all qemu-system-* binaries, see qemu(1) for documentation of all options.

Usually, if an option has many possible values, you can use

$ qemu-system-x86_64 option help
to list all possible values. If it supports properties, you can use

$ qemu-system-x86_64 option value,help
to list all available properties.

For example:

$ qemu-system-x86_64 -machine help
$ qemu-system-x86_64 -machine q35,help
$ qemu-system-x86_64 -device help
$ qemu-system-x86_64 -device qxl,help
You can use these methods and the qemu(1) documentation to understand the options used in follow sections.

By default, QEMU will show the virtual machine's video output in a window. One thing to keep in mind: when you click inside the QEMU window, the mouse pointer is grabbed. To release it, press Ctrl+Alt+g.

Warning: QEMU should never be run as root. If you must launch it in a script as root, you should use the -runas option to make QEMU drop root privileges.
Enabling KVM
KVM (Kernel-based Virtual Machine) full virtualization must be supported by your Linux kernel and your hardware, and necessary kernel modules must be loaded. See KVM for more information.

To start QEMU in KVM mode, append -accel kvm to the additional start options. To check if KVM is enabled for a running virtual machine, enter the #QEMU monitor and type info kvm.

Note:
The argument accel=kvm of the -machine option is equivalent to the -enable-kvm or the -accel kvm option.
CPU model host requires KVM.
If you start your virtual machine with a GUI tool and experience very bad performance, you should check for proper KVM support, as QEMU may be falling back to software emulation.
KVM needs to be enabled in order to start Windows 7 or Windows 8 properly without a blue screen.
Enabling IOMMU (Intel VT-d/AMD-Vi) support
First enable IOMMU, see PCI passthrough via OVMF#Setting up IOMMU.

Add -device intel-iommu to create the IOMMU device:

$ qemu-system-x86_64 -enable-kvm -machine q35 -device intel-iommu -cpu host ..
Note: On Intel CPU based systems creating an IOMMU device in a QEMU guest with -device intel-iommu will disable PCI passthrough with an error like:
Device at bus pcie.0 addr 09.0 requires iommu notifier which is currently not supported by intel-iommu emulation
While adding the kernel parameter intel_iommu=on is still needed for remapping IO (e.g. PCI passthrough with vfio-pci), -device intel-iommu should not be set if PCI passthrough is required.
Booting in UEFI mode
The default firmware used by QEMU is SeaBIOS, which is a Legacy BIOS implementation. QEMU uses /usr/share/qemu/bios-256k.bin (provided by the seabios package) as a default read-only (ROM) image. You can use the -bios argument to select another firmware file. However, UEFI requires writable memory to work properly, so you need to emulate PC System Flash instead.

OVMF is a TianoCore project to enable UEFI support for Virtual Machines. It can be installed with the edk2-ovmf package.

There are two ways to use OVMF as a firmware. The first is to copy /usr/share/edk2/x64/OVMF.4m.fd, make it writable and use as a pflash drive:

-drive if=pflash,format=raw,file=/copy/of/OVMF.4m.fd
All changes to the UEFI settings will be saved directly to this file.

Another and more preferable way is to split OVMF into two files. The first one will be read-only and store the firmware executable, and the second one will be used as a writable variable store. The advantage is that you can use the firmware file directly without copying, so it will be updated automatically by pacman.

Use /usr/share/edk2/x64/OVMF_CODE.4m.fd as a first read-only pflash drive. Copy /usr/share/edk2/x64/OVMF_VARS.4m.fd, make it writable and use as a second writable pflash drive:

-drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.4m.fd \
-drive if=pflash,format=raw,file=/copy/of/OVMF_VARS.4m.fd
Enabling Secure Boot
The first requirement to enabling Secure Boot in a VM is to use the q35 machine type, and replace /usr/share/edk2/x64/OVMF_CODE.4m.fd with /usr/share/edk2/x64/OVMF_CODE.secboot.4m.fd.

The second requirement is to use an OVMF_VARS file that has Secure Boot keys installed, which is not provided by the upstream project[1].

Unlike some other distributions, Arch does not yet provide (as of 2024-12-06) its own /usr/share/edk2/x64/OVMF_VARS.secboot.4m.fd file containing pre-enrolled keys: see archlinux/packaging/packages/edk2#1.

A simple workaround is to use Fedora's version, as mentioned in this forum post:

download a recent noarch RPM of Fedora's edk2-ovmf for x86_64 from the builds list (direct link to latest F42 version as of 2024-12-06)
extract the archive and convert the desired file to a suitable format[2]:
$ qemu-img convert -O raw -f qcow2 edk2-ovmf-*.noarch/usr/share/edk2/ovmf/OVMF_VARS_4M.secboot.qcow2 OVMF_VARS_4M.secboot.fd
Now you can make copies of the created OVMF_VARS_4M.secboot.fd and use them for your Secure Boot-enabled VMs.

See also KVM#Secure Boot for how to manually enroll keys into a firmware image.

Tip: A more cumbersome, although slightly more "upstream-aligned" workaround to manually enroll keys in a vanilla OVMF_VARS file is described in this forum post.
Trusted Platform Module emulation
QEMU can emulate Trusted Platform Module, which is required by some systems such as Windows 11 (which requires TPM 2.0).

Install the swtpm package, which provides a software TPM implementation. Create some directory for storing TPM data (/path/to/mytpm will be used as an example). Run this command to start the emulator:

$ swtpm socket --tpm2 --tpmstate dir=/path/to/mytpm --ctrl type=unixio,path=/path/to/mytpm/swtpm-sock
/path/to/mytpm/swtpm-sock will be created by swtpm: this is a UNIX socket to which QEMU will connect. You can put it in any directory.

By default, swtpm starts a TPM version 1.2 emulator. The --tpm2 option enables TPM 2.0 emulation.

Finally, add the following options to QEMU:

-chardev socket,id=chrtpm,path=/path/to/mytpm/swtpm-sock \
-tpmdev emulator,id=tpm0,chardev=chrtpm \
-device tpm-tis,tpmdev=tpm0
and TPM will be available inside the virtual machine. After shutting down the virtual machine, swtpm will be automatically terminated.

See the QEMU documentation for more information.

If guest OS still does not recognize the TPM device, try to adjust CPU Models and Topology options. It might cause problem.

Communication between host and guest
Network
Data can be shared between the host and guest OS using any network protocol that can transfer files, such as NFS, SMB, NBD, HTTP, FTP, or SSH, provided that you have set up the network appropriately and enabled the appropriate services.

The default SLIRP-based user-mode networking allows the guest to access the host OS at the IP address 10.0.2.2. Any servers that you are running on your host OS, such as a SSH server or SMB server, will be accessible at this IP address. So on the guests, you can mount directories exported on the host via SMB or NFS, or you can access the host's HTTP server, etc. It will not be possible for the host OS to access servers running on the guest OS, but this can be done with other network configurations (see #Tap networking with QEMU).

QEMU's port forwarding
Note: QEMU's port forwarding is IPv4-only. IPv6 port forwarding is not implemented and the last patches were proposed in 2018 [3]. If you need full IPv6 support, check #passt
QEMU can forward ports from the host to the guest to enable e.g. connecting from the host to an SSH server running on the guest.

For example, to bind port 60022 on the host with port 22 (SSH) on the guest, start QEMU with a command like:

$ qemu-system-x86_64 disk_image -nic user,hostfwd=tcp::60022-:22
Make sure the sshd is running on the guest and connect with:

$ ssh guest-user@127.0.0.1 -p 60022
You can use SSHFS to mount the guest's file system at the host for shared read and write access.

To forward several ports, you just repeat the hostfwd in the -nic argument, e.g. for VNC's port:

$ qemu-system-x86_64 disk_image -nic user,hostfwd=tcp::60022-:22,hostfwd=tcp::5900-:5900
Accessing SSH via vsock
A secure and convenient way to connect to the VM is to use SSH over vsock(7). Your VM needs to be systemd-based for this to work out of the box.

First, launch QEMU with a special device:

-device vhost-vsock-pci,id=vhost-vsock-pci0,guest-cid=555
The cid needs to be picked by the user to be a valid 32-bit number (see vsock(7)). When systemd detects the VM has been launched with a vhost-vsock device, it will automatically launch an SSH server via systemd-ssh-generator.

You can then connect to the VM like this:

$ ssh user@vsock/555
This works because of /etc/ssh/ssh_config.d/20-systemd-ssh-proxy.conf which tells your SSH client to use systemd-ssh-proxy to allow SSH to use vsock.

Furthermore, using systemd.system-credentials(7) we can inject an authorized keys file for the root user which is very convenient in case we are trying to run a downloaded image. This can be done like so:

-smbios type=11,value=io.systemd.credential.binary:ssh.authorized_keys.root=c3NoLWVkMjU1MTkgQUFBQUMzTnphQzFsWkRJMU5URTVBQUFBSU9sVFE4ejlpeWxoMTMreCtFVFJ1R1JEaHpIVVRnaCt2ekJLOGY3TEl5eTQ=
The public key line has be provided as a base64-encoded string. This can be done like so:

echo "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOlTQ8z9iylh13+x+ETRuGRDhzHUTgh+vzBK8f7LIyy4" | base64
The same mechanism via -smbios type=11,value=io.systemd... can be used to inject a variety of other magical variables that will get acted on by systemd. See also systemd docs: System and Service Credentials.

QEMU's built-in SMB server
QEMU's documentation says it has a "built-in" SMB server, but actually it just starts up Samba on the host with an automatically generated smb.conf file located in /tmp/qemu-smb.random_string and makes it accessible to the guest at a different IP address (10.0.2.4 by default). This only works for user networking, and is useful when you do not want to start the normal Samba service on the host, which the guest can also access if you have set up shares on it.

Only a single directory can be set as shared with the option smb=, but adding more directories (even while the virtual machine is running) could be as easy as creating symbolic links in the shared directory if QEMU configured SMB to follow symbolic links. It does not do so, but the configuration of the running SMB server can be changed as described below.

Samba must be installed on the host. To enable this feature, start QEMU with a command like:

$ qemu-system-x86_64 -nic user,id=nic0,smb=shared_dir_path disk_image
where shared_dir_path is a directory that you want to share between the guest and host.

Then, in the guest, you will be able to access the shared directory on the host 10.0.2.4 with the share name "qemu". For example, in Windows Explorer you would go to \\10.0.2.4\qemu.

Note:
If you are using sharing options multiple times like -net user,smb=shared_dir_path1 -net user,smb=shared_dir_path2 or -net user,smb=shared_dir_path1,smb=shared_dir_path2 then it will share only the last defined one.
If you cannot access the shared folder and the guest system is Windows, check that the NetBIOS protocol is enabled and that a firewall does not block ports used by the NetBIOS protocol.
If you cannot access the shared folder and the guest system is Windows 10 Enterprise or Education or Windows Server 2016, enable guest access.
If you use #Tap networking with QEMU, use -device virtio-net,netdev=vmnic -netdev user,id=vmnic,smb=shared_dir_path to get SMB.
One way to share multiple directories and to add or remove them while the virtual machine is running, is to share an empty directory and create/remove symbolic links to the directories in the shared directory. For this to work, the configuration of the running SMB server can be changed with the following script, which also allows the execution of files on the guest that are not set executable on the host:

#!/bin/sh
eval $(ps h -C smbd -o pid,args | grep /tmp/qemu-smb | gawk '{print "pid="$1";conf="$6}')
echo "[global]
allow insecure wide links = yes
[qemu]
follow symlinks = yes
wide links = yes
acl allow execute always = yes" >> "$conf"
# in case the change is not detected automatically:
smbcontrol --configfile="$conf" "$pid" reload-config
This can be applied to the running server started by qemu only after the guest has connected to the network drive the first time. An alternative to this method is to add additional shares to the configuration file like so:

echo "[myshare]
path=another_path
read only=no
guest ok=yes
force user=username" >> $conf
This share will be available on the guest as \\10.0.2.4\myshare.

Host file sharing with 9pfs VirtFS
Note: 9pfs is fairly slow as it was not specifically developed for high-performance local VM usage. Users are advised to look into #Host file sharing with virtiofsd instead which was specifically made with VM performance in mind.
See the QEMU documentation.

Host file sharing with virtiofsd
virtiofsd is shipped with the virtiofsd package. It's a modern and high-performance way to conveniently share files between host and guest. See the online documentation or /usr/share/doc/virtiofsd/README.md for a full list of available options.

You can choose to either run virtiofsd as root or as a regular user.

Running virtiofsd as a regular user
First, make sure that there is a subuid(5) and subgid(5) configuration entry for the user that will execute virtiofsd. See also the relevant section in the Podman article.

Then, start virtiofsd:

$ unshare -r --map-auto -- /usr/lib/virtiofsd --socket-path=/tmp/vm-share.sock --shared-dir /tmp/vm-share --sandbox chroot
unshare -r causes the command after it to be launched in a new user namespace with the current user getting mapped to root in the new command. This is important because virtiofsd expects to be running as root from its point of view.
/tmp/vm-share.sock is a socket file
/tmp/vm-share is a shared directory between the host and the guest virtual machine
Running virtiofsd as root
Add the user that runs QEMU to the kvm user group, because it needs to access the virtiofsd socket. You might have to logout for change to take effect.

Start virtiofsd as root:

# /usr/lib/virtiofsd --socket-path /tmp/vm-share.sock --socket-group kvm --shared-dir /tmp/vm-share
where

/tmp/vm-share.sock is a socket file
/tmp/vm-share is a shared directory between the host and the guest virtual machine
Launching QEMU
Add the following configuration options when starting the virtual machine:

-m 4G
-object memory-backend-memfd,id=mem,size=4G,share=on
-numa node,memdev=mem \
-chardev socket,id=char0,path=/tmp/vm-share.sock
-device vhost-user-fs-pci,chardev=char0,tag=myfs
where

size=4G must match the size specified with -m 4G option
/tmp/vm-share.sock points to socket file started earlier
myfs is an identifier that you will use later in the guest to mount the share
Boot rootfs directly
You may also boot a rootfs directly via virtiofsd. In addition to the above arguments, append:

-kernel /path/to/vmlinux
-initrd /path/to/initramfs
-append 'rootfstype=virtiofs root=myfs rootflags=rw,noatime'
Using the share in a Linux guest
Once logged into the guest as root, you can simply mount the share on any modern distribution:

# mount -t virtiofs myfs /mnt
This directory should now be shared between host and guest.

Using the share in a Windows guest
See relevant Windows section.

Mounting a partition of the guest on the host
It can be useful to mount a drive image under the host system, it can be a way to transfer files in and out of the guest. This should be done when the virtual machine is not running.

The procedure to mount the drive on the host depends on the type of qemu image, raw or qcow2. We detail thereafter the steps to mount a drive in the two formats in #Mounting a partition from a raw image and #Mounting a partition from a qcow2 image. For the full documentation see Wikibooks:QEMU/Images#Mounting an image on the host.

Warning: You must unmount the partitions before running the virtual machine again. Otherwise, data corruption is very likely to occur.
Mounting a partition from a raw image
It is possible to mount partitions that are inside a raw disk image file by setting them up as loopback devices.

With manually specifying byte offset
One way to mount a disk image partition is to mount the disk image at a certain offset using a command like the following:

# mount -o loop,offset=32256 disk_image mountpoint
The offset=32256 option is actually passed to the losetup program to set up a loopback device that starts at byte offset 32256 of the file and continues to the end. This loopback device is then mounted. You may also use the sizelimit option to specify the exact size of the partition, but this is usually unnecessary.

Depending on your disk image, the needed partition may not start at offset 32256. Run fdisk -l disk_image to see the partitions in the image. fdisk gives the start and end offsets in 512-byte sectors, so multiply by 512 to get the correct offset to pass to mount.

With loop module autodetecting partitions
The Linux loop driver actually supports partitions in loopback devices, but it is disabled by default. To enable it, do the following:

Get rid of all your loopback devices (unmount all mounted images, etc.).
Unload the loop kernel module, and load it with the max_part=15 parameter set. Additionally, the maximum number of loop devices can be controlled with the max_loop parameter.
Tip: You can put an entry in /etc/modprobe.d to load the loop module with max_part=15 every time, or you can put loop.max_part=15 on the kernel command-line, depending on whether you have the loop.ko module built into your kernel or not.
Set up your image as a loopback device:

# losetup -f -P disk_image
Then, if the device created was /dev/loop0, additional devices /dev/loop0pX will have been automatically created, where X is the number of the partition. These partition loopback devices can be mounted directly. For example:

# mount /dev/loop0p1 mountpoint
To mount the disk image with udisksctl, see Udisks#Mount loop devices.

With kpartx
kpartx from the multipath-tools package can read a partition table on a device and create a new device for each partition. For example:

# kpartx -a disk_image
This will setup the loopback device and create the necessary partition(s) device(s) in /dev/mapper/.

Mounting a partition from a qcow2 image
We will use qemu-nbd, which lets us use the NBD (network block device) protocol to share the disk image.

First, we need the nbd module loaded:

# modprobe nbd max_part=16
Then, we can share the disk and create the device entries:

# qemu-nbd -c /dev/nbd0 /path/to/image.qcow2
Discover the partitions:

# partprobe /dev/nbd0
fdisk can be used to get information regarding the different partitions in nbd0:

# fdisk -l /dev/nbd0
Disk /dev/nbd0: 25.2 GiB, 27074281472 bytes, 52879456 sectors
Units: sectors of 1 * 512 = 512 bytes
Sector size (logical/physical): 512 bytes / 512 bytes
I/O size (minimum/optimal): 512 bytes / 512 bytes
Disklabel type: dos
Disk identifier: 0xa6a4d542

Device      Boot   Start      End  Sectors  Size Id Type
/dev/nbd0p1 *       2048  1026047  1024000  500M  7 HPFS/NTFS/exFAT
/dev/nbd0p2      1026048 52877311 51851264 24.7G  7 HPFS/NTFS/exFAT
Then mount any partition of the drive image, for example the partition 2:

# mount /dev/nbd0p2 mountpoint
After the usage, it is important to unmount the image and reverse previous steps, i.e. unmount the partition and disconnect the nbd device:

# umount mountpoint
# qemu-nbd -d /dev/nbd0
Networking
This article or section needs language, wiki syntax or style improvements. See Help:Style for reference.

Reason: Network topologies (sections #Host-only networking, #Internal networking and info spread out across other sections) should not be described alongside the various virtual interfaces implementations, such as #User-mode networking, #Tap networking with QEMU, QEMU/Advanced networking#Networking with VDE2. (Discuss in Talk:QEMU)
The performance of virtual networking should be better with tap devices and bridges than with user-mode networking or vde because tap devices and bridges are implemented in-kernel.

In addition, networking performance can be improved by assigning virtual machines a virtio network device rather than the default emulation of an e1000 NIC. See #Using virtio drivers for more information.

Link-level address caveat
By giving the -net nic argument to QEMU, it will, by default, assign a virtual machine a network interface with the link-level address 52:54:00:12:34:56. However, when using bridged networking with multiple virtual machines, it is essential that each virtual machine has a unique link-level (MAC) address on the virtual machine side of the tap device. Otherwise, the bridge will not work correctly, because it will receive packets from multiple sources that have the same link-level address. This problem occurs even if the tap devices themselves have unique link-level addresses because the source link-level address is not rewritten as packets pass through the tap device.

Make sure that each virtual machine has a unique link-level address, but it should always start with 52:54:. Use the following option, replace X with arbitrary hexadecimal digit:

$ qemu-system-x86_64 -net nic,macaddr=52:54:XX:XX:XX:XX -net vde disk_image
Generating unique link-level addresses can be done in several ways:

Manually specify unique link-level address for each NIC. The benefit is that the DHCP server will assign the same IP address each time the virtual machine is run, but it is unusable for large number of virtual machines.
Generate random link-level address each time the virtual machine is run. Practically zero probability of collisions, but the downside is that the DHCP server will assign a different IP address each time. You can use the following command in a script to generate random link-level address in a macaddr variable:
printf -v macaddr "52:54:%02x:%02x:%02x:%02x" $(( $RANDOM & 0xff)) $(( $RANDOM & 0xff )) $(( $RANDOM & 0xff)) $(( $RANDOM & 0xff ))
qemu-system-x86_64 -net nic,macaddr="$macaddr" -net vde disk_image
Use the following script qemu-mac-hasher.py to generate the link-level address from the virtual machine name using a hashing function. Given that the names of virtual machines are unique, this method combines the benefits of the aforementioned methods: it generates the same link-level address each time the script is run, yet it preserves the practically zero probability of collisions.
qemu-mac-hasher.py
#!/usr/bin/env python
# usage: qemu-mac-hasher.py <VMName>

import sys
import zlib

crc = str(hex(zlib.crc32(sys.argv[1].encode("utf-8")))).replace("x", "")[-8:]
print("52:54:%s%s:%s%s:%s%s:%s%s" % tuple(crc))
In a script, you can use for example:

vm_name="VM Name"
qemu-system-x86_64 -name "$vm_name" -net nic,macaddr=$(qemu-mac-hasher.py "$vm_name") -net vde disk_image
User-mode networking
SLIRP
By default, without any -netdev arguments, QEMU will use SLIRP-based user-mode networking with a built-in DHCP server. Your virtual machines will be assigned an IP address when they run their DHCP client, and they will be able to access the physical host's network through IP masquerading done by QEMU.

Note: ICMPv6 will not work, as support for it is not implemented: Slirp: external icmpv6 not supported yet. Pinging an IPv6 address will not work.
This default configuration allows your virtual machines to easily access the Internet, provided that the host is connected to it, but the virtual machines will not be directly visible on the external network, nor will virtual machines be able to talk to each other if you start up more than one concurrently.

QEMU's user-mode networking can offer more capabilities such as built-in TFTP or SMB servers, redirecting host ports to the guest (for example to allow SSH connections to the guest) or attaching guests to VLANs so that they can talk to each other. See the QEMU documentation on the -net user flag for more details.

However, SLIRP-based user-mode networking has limitations in both utility and performance. More advanced network configurations require the use of tap devices or other methods.

Note: If the host system uses systemd-networkd, make sure to symlink the /etc/resolv.conf file as described in systemd-networkd#Required services and setup, otherwise the DNS lookup in the guest system will not work.
Tip:
To use the virtio driver with user-mode networking, the option is: -nic user,model=virtio-net-pci.
You can isolate user-mode networking from the host and the outside world by adding restrict=y, for example: -net user,restrict=y
passt
Users can choose to use passt-based user-mode networking. passt has several advantages over SLIRP such as better performance, full IPv6 support (including ICMPv6), better security, and more control.

To get started, install passt. There are two ways to launch it: Either via socket-based communication or via shared vhost-user. The latter method has better performance.

For the socket-based way, first launch passt:

$ passt -f
Then, for your QEMU command, add these parameters:

-device virtio-net-pci,netdev=s
-netdev stream,id=s,server=off,addr.type=unix,addr.path=/tmp/passt_1.socket
For the vhost-user way, launch passt with --vhost-user

$ passt -f --vhost-user
Then, for your QEMU command, add these parameters:

-m 4G
-chardev socket,id=chr0,path=/tmp/passt_1.socket
-netdev vhost-user,id=netdev0,chardev=chr0
-device virtio-net,netdev=netdev0
-object memory-backend-memfd,id=memfd0,share=on,size=4G
-numa node,memdev=memfd0
Notice the memory sizes of -m 4G and size=4G have to match exactly.

Tap networking with QEMU
Tap devices are a Linux kernel feature that allows you to create virtual network interfaces that appear as real network interfaces. Packets sent to a tap interface are delivered to a userspace program, such as QEMU, that has bound itself to the interface.

QEMU can use tap networking for a virtual machine so that packets sent to the tap interface will be sent to the virtual machine and appear as coming from a network interface (usually an Ethernet interface) in the virtual machine. Conversely, everything that the virtual machine sends through its network interface will appear on the tap interface.

Tap devices are supported by the Linux bridge drivers, so it is possible to bridge together tap devices with each other and possibly with other host interfaces such as eth0. This is desirable if you want your virtual machines to be able to talk to each other, or if you want other machines on your LAN to be able to talk to the virtual machines.

Warning: If you bridge together tap device and some host interface, such as eth0, your virtual machines will appear directly on the external network, which will expose them to possible attack. Depending on what resources your virtual machines have access to, you may need to take all the precautions you normally would take in securing a computer to secure your virtual machines. If the risk is too great, virtual machines have little resources or you set up multiple virtual machines, a better solution might be to use host-only networking and set up NAT. In this case you only need one firewall on the host instead of multiple firewalls for each guest.
As indicated in the user-mode networking section, tap devices offer higher networking performance than user-mode. If the guest OS supports virtio network driver, then the networking performance will be increased considerably as well. Supposing the use of the tap0 device, that the virtio driver is used on the guest, and that no scripts are used to help start/stop networking, next is part of the qemu command one should see:

-device virtio-net,netdev=network0 -netdev tap,id=network0,ifname=tap0,script=no,downscript=no
But if already using a tap device with virtio networking driver, one can even boost the networking performance by enabling vhost, like:

-device virtio-net,netdev=network0 -netdev tap,id=network0,ifname=tap0,script=no,downscript=no,vhost=on
See [4] for more information.

Host-only networking
If the bridge is given an IP address and traffic destined for it is allowed, but no real interface (e.g. eth0) is connected to the bridge, then the virtual machines will be able to talk to each other and the host system. However, they will not be able to talk to anything on the external network, provided that you do not set up IP masquerading on the physical host. This configuration is called host-only networking by other virtualization software such as VirtualBox.

Tip:
If you want to set up IP masquerading, e.g. NAT for virtual machines, see the Internet sharing#Enable NAT page.
See Network bridge for information on creating bridge.
You may want to have a DHCP server running on the bridge interface to service the virtual network. For example, to use the 172.20.0.1/16 subnet with dnsmasq as the DHCP server:
# ip addr add 172.20.0.1/16 dev br0
# ip link set br0 up
# dnsmasq -C /dev/null --interface=br0 --bind-interfaces --dhcp-range=172.20.0.2,172.20.255.254
Internal networking
If you do not give the bridge an IP address and add an iptables rule to drop all traffic to the bridge in the INPUT chain, then the virtual machines will be able to talk to each other, but not to the physical host or to the outside network. This configuration is called internal networking by other virtualization software such as VirtualBox. You will need to either assign static IP addresses to the virtual machines or run a DHCP server on one of them.

By default iptables would drop packets in the bridge network. You may need to use such iptables rule to allow packets in a bridged network:

# iptables -I FORWARD -m physdev --physdev-is-bridged -j ACCEPT
Bridged networking using qemu-bridge-helper
This method does not require a start-up script and readily accommodates multiple taps and multiple bridges. It uses /usr/lib/qemu/qemu-bridge-helper binary, which allows creating tap devices on an existing bridge.

Tip:
See Network bridge for information on creating bridge.
See https://wiki.qemu.org/Features/HelperNetworking for more information on QEMU's network helper.
First, create a configuration file containing the names of all bridges to be used by QEMU:

/etc/qemu/bridge.conf
allow br0
allow br1
...
Make sure /etc/qemu/ has 755 permissions. QEMU issues and GNS3 issues may arise if this is not the case.

Now start the virtual machine; the most basic usage to run QEMU with the default network helper and default bridge br0:

$ qemu-system-x86_64 -nic bridge [...]
Using the bridge br1 and the virtio driver:

$ qemu-system-x86_64 -nic bridge,br=br1,model=virtio-net-pci [...]
Advanced network configuration
If you need more control over your virtual machine's networking or you have very specific needs that arent covered in the previous setctions, see QEMU/Advanced networking.

Shorthand configuration
If you are using QEMU with various networking options a lot, you probably have created a lot of -netdev and -device argument pairs, which gets quite repetitive. You can instead use the -nic argument to combine -netdev and -device together, so that, for example, these arguments:

-netdev tap,id=network0,ifname=tap0,script=no,downscript=no,vhost=on -device virtio-net-pci,netdev=network0
become:

-nic tap,script=no,downscript=no,vhost=on,model=virtio-net-pci
Notice the lack of network IDs, and that the device was created with model=. The first half of the -nic parameters are -netdev parameters, whereas the second half (after model=) are related with the device. The same parameters (for example, smb=) are used. To completely disable the networking use -nic none.

See QEMU networking documentation for more information on parameters you can use.

Graphics card
QEMU can emulate a standard graphics card text mode using -display curses command line option. This allows to type text and see text output directly inside a text terminal. Alternatively, -nographic serves a similar purpose.

QEMU can emulate several types of VGA card. The card type is passed in the -vga type command line option and can be std, qxl, vmware, virtio, cirrus or none.

std
With -vga std you can get a resolution of up to 2560 x 1600 pixels without requiring guest drivers. This is the default since QEMU 2.2.

qxl
QXL is a paravirtual graphics driver with 2D support. To use it, pass the -vga qxl option and install drivers in the guest. You may want to use #SPICE for improved graphical performance when using QXL.

On Linux guests, the qxl and bochs_drm kernel modules must be loaded in order to gain a decent performance.

Default VGA memory size for QXL devices is 16M which is sufficient to drive resolutions approximately up to QHD (2560x1440). To enable higher resolutions, increase vga_memmb.

vmware
Although it is a bit buggy, it performs better than std and cirrus. Install the VMware drivers xf86-video-vmware and xf86-input-vmmouse for Arch Linux guests.

virtio
virtio-vga / virtio-gpu is a paravirtual 3D graphics driver based on virgl. It's mature, currently supporting only Linux guests with mesa compiled with the option gallium-drivers=virgl.

To enable 3D acceleration on the guest system, select this vga with -device virtio-vga-gl and enable the OpenGL context in the display device with -display sdl,gl=on or -display gtk,gl=on for the SDL and GTK display output respectively. Successful configuration can be confirmed looking at the kernel log in the guest:

# dmesg | grep drm 
[drm] pci: virtio-vga detected
[drm] virgl 3d acceleration enabled
cirrus
The cirrus graphical adapter was the default before 2.2. It should not be used on modern systems.

none
This is like a PC that has no VGA card at all. You would not even be able to access it with the -vnc option. Also, this is different from the -nographic option which lets QEMU emulate a VGA card, but disables the SDL display.

SPICE
The SPICE project aims to provide a complete open source solution for remote access to virtual machines in a seamless way.

Enabling SPICE support on the host
The following is an example of booting with SPICE as the remote desktop protocol, including the support for copy and paste from host:

$ qemu-system-x86_64 -vga qxl -device virtio-serial-pci -spice port=5930,disable-ticketing=on -device virtserialport,chardev=spicechannel0,name=com.redhat.spice.0 -chardev spicevmc,id=spicechannel0,name=vdagent
The parameters have the following meaning:

-device virtio-serial-pci adds a virtio-serial device
-spice port=5930,disable-ticketing=on set TCP port 5930 for spice channels listening and allow client to connect without authentication
Tip: Using Unix sockets instead of TCP ports does not involve using network stack on the host system. It does not imply that packets are encapsulated and decapsulated to use the network and the related protocol. The sockets are identified solely by the inodes on the hard drive. It is therefore considered better for performance. Use instead -spice unix=on,addr=/tmp/vm_spice.socket,disable-ticketing=on.
-device virtserialport,chardev=spicechannel0,name=com.redhat.spice.0 opens a port for spice vdagent in the virtio-serial device,
-chardev spicevmc,id=spicechannel0,name=vdagent adds a spicevmc chardev for that port. It is important that the chardev= option of the virtserialport device matches the id= option given to the chardev option (spicechannel0 in this example). It is also important that the port name is com.redhat.spice.0, because that is the namespace where vdagent is looking for in the guest. And finally, specify name=vdagent so that spice knows what this channel is for.
Connecting to the guest with a SPICE client
A SPICE client is necessary to connect to the guest. In Arch, the following clients are available:

virt-viewer — SPICE client recommended by the protocol developers, a subset of the virt-manager project.
https://virt-manager.org/ || virt-viewer
spice-gtk — SPICE GTK client, a subset of the SPICE project. Embedded into other applications as a widget.
https://www.spice-space.org/ || spice-gtk
For clients that run on smartphone or on other platforms, refer to the Other clients section in spice-space download.

Manually running a SPICE client
One way of connecting to a guest listening on Unix socket /tmp/vm_spice.socket is to manually run the SPICE client using $ remote-viewer spice+unix:///tmp/vm_spice.socket or $ spicy --uri="spice+unix:///tmp/vm_spice.socket", depending on the desired client. Since QEMU in SPICE mode acts similarly to a remote desktop server, it may be more convenient to run QEMU in daemon mode with the -daemonize parameter.

Tip: To connect to the guest through SSH tunneling, the following type of command can be used:
$ ssh -fL 5999:localhost:5930 my.domain.org sleep 10; spicy -h 127.0.0.1 -p 5999
This example connects spicy to the local port 5999 which is forwarded through SSH to the guest's SPICE server located at the address my.domain.org, port 5930. Note the -f option that requests ssh to execute the command sleep 10 in the background. This way, the ssh session runs while the client is active and auto-closes once the client ends.
Running a SPICE client with QEMU
QEMU can automatically start a SPICE client with an appropriate socket, if the display is set to SPICE with the -display spice-app parameter. This will use the system's default SPICE client as the viewer, determined by your mimeapps.list files.

Enabling SPICE support on the guest
For Arch Linux guests, for improved support for multiple monitors or clipboard sharing, the following packages should be installed:

spice-vdagent: Spice agent xorg client that enables copy and paste between client and X-session and more. (Refer to this issue, until fixed, for workarounds to get this to work on non-GNOME desktops.)
xf86-video-qxl: Xorg X11 qxl video driver
x-resizeAUR: Desktop environments other than GNOME do not react automatically when the SPICE client window is resized. This package uses a udev rule and xrandr to implement auto-resizing for all X11-based desktop environments and window managers.
For guests under other operating systems, refer to the Guest section in spice-space download.

Password authentication with SPICE
If you want to enable password authentication with SPICE you need to remove disable-ticketing from the -spice argument and instead add password=yourpassword. For example:

$ qemu-system-x86_64 -vga qxl -spice port=5900,password=yourpassword -device virtio-serial-pci -device virtserialport,chardev=spicechannel0,name=com.redhat.spice.0 -chardev spicevmc,id=spicechannel0,name=vdagent
Your SPICE client should now ask for the password to be able to connect to the SPICE server.

TLS encrypted communication with SPICE
You can also configure TLS encryption for communicating with the SPICE server. First, you need to have a directory which contains the following files (the names must be exactly as indicated):

ca-cert.pem: the CA master certificate.
server-cert.pem: the server certificate signed with ca-cert.pem.
server-key.pem: the server private key.
An example of generation of self-signed certificates with your own generated CA for your server is shown in the Spice User Manual.

Afterwards, you can run QEMU with SPICE as explained above but using the following -spice argument: -spice tls-port=5901,password=yourpassword,x509-dir=/path/to/pki_certs, where /path/to/pki_certs is the directory path that contains the three needed files shown earlier.

It is now possible to connect to the server using virt-viewer:

$ remote-viewer spice://hostname?tls-port=5901 --spice-ca-file=/path/to/ca-cert.pem --spice-host-subject="C=XX,L=city,O=organization,CN=hostname" --spice-secure-channels=all
Keep in mind that the --spice-host-subject parameter needs to be set according to your server-cert.pem subject. You also need to copy ca-cert.pem to every client to verify the server certificate.

Tip: You can get the subject line of the server certificate in the correct format for --spice-host-subject (with entries separated by commas) using the following command:
$ openssl x509 -noout -subject -in server-cert.pem | cut -d' ' -f2- | sed 's/\///' | sed 's/\//,/g'
The equivalent spice-gtk command is:

$ spicy -h hostname -s 5901 --spice-ca-file=ca-cert.pem --spice-host-subject="C=XX,L=city,O=organization,CN=hostname" --spice-secure-channels=all
VNC
One can add the -vnc :X option to have QEMU redirect the VGA display to the VNC session. Substitute X for the number of the display (0 will then listen on 5900, 1 on 5901...).

$ qemu-system-x86_64 -vnc :0
An example is also provided in the #Starting QEMU virtual machines on boot section.

Warning: The default VNC server setup does not use any form of authentication. Any user can connect from any host.
Basic password authentication
An access password can be setup easily by using the password option. The password must be indicated in the QEMU monitor and connection is only possible once the password is provided.

$ qemu-system-x86_64 -vnc :0,password -monitor stdio
In the QEMU monitor, password is set using the command change vnc password and then indicating the password.

The following command line directly runs vnc with a password:

$ printf "change vnc password\n%s\n" MYPASSWORD | qemu-system-x86_64 -vnc :0,password -monitor stdio
Note: The password is limited to 8 characters and can be guessed through brute force attack. More elaborated protection is strongly recommended for public network.
Audio
Creating an audio backend
The -audiodev flag sets the audio backend driver on the host and its options.

To list availabe audio backend drivers:

$ qemu-system-x86_64 -audiodev help
Their optional settings are detailed in the qemu(1) man page.

At the bare minimum, one need to choose an audio backend and set an id, for PulseAudio for example:

-audiodev pa,id=snd0
Using the audio backend
Intel HD Audio
For Intel HD Audio emulation, add both controller and codec devices. To list the available Intel HDA Audio devices:

$ qemu-system-x86_64 -device help | grep hda
Add the audio controller:

-device ich9-intel-hda
Also, add the audio codec and map it to a host audio backend id:

-device hda-output,audiodev=snd0
Intel 82801AA AC97
For AC97 emulation just add the audio card device and map it to a host audio backend id:

-device AC97,audiodev=snd0
Note:
If the audiodev backend is not provided, QEMU looks up for it and adds it automatically, this only works for a single audiodev. For example -device intel-hda -device hda-duplex will emulate intel-hda on the guest using the default audiodev backend.
Video graphics card emulated drivers for the guest machine may also cause a problem with the sound quality. Test one by one to make it work. You can list possible options with qemu-system-x86_64 -h | grep vga.
VirtIO sound
VirtIO sound is also available since QEMU 8.2.0. The usage is:

-device virtio-sound-pci,audiodev=my_audiodev -audiodev alsa,id=my_audiodev
More information can be found in QEMU documentation.

Using virtio drivers
QEMU offers guests the ability to use paravirtualized block and network devices using the virtio drivers, which provide better performance and lower overhead.

A virtio block device requires the option -drive for passing a disk image, with parameter if=virtio:
$ qemu-system-x86_64 -drive file=disk_image,if=virtio
Almost the same goes for the network:
$ qemu-system-x86_64 -nic user,model=virtio-net-pci
Note: This will only work if the guest machine has drivers for virtio devices. Linux does, and the required drivers are included in Arch Linux, but there is no guarantee that virtio devices will work with other operating systems.
Preparing an Arch Linux guest
To use virtio devices after an Arch Linux guest has been installed, the following modules must be loaded in the guest: virtio, virtio_pci, virtio_blk, virtio_net, and virtio_ring. For 32-bit guests, the specific "virtio" module is not necessary.

If you want to boot from a virtio disk, the initial ramdisk must contain the necessary modules. By default, this is handled by mkinitcpio's autodetect hook. Otherwise use the MODULES array in /etc/mkinitcpio.conf to include the necessary modules and rebuild the initial ramdisk.

/etc/mkinitcpio.conf
MODULES=(virtio virtio_blk virtio_pci virtio_net)
Virtio disks are recognized with the prefix v (e.g. vda, vdb, etc.); therefore, changes must be made in at least /etc/fstab and /boot/grub/grub.cfg when booting from a virtio disk.

Tip: When referencing disks by UUID in both /etc/fstab and boot loader, nothing has to be done.
Further information on paravirtualization with KVM can be found here.

You might also want to install qemu-guest-agent to implement support for QMP commands that will enhance the hypervisor management capabilities.

Memory ballooning
In order to allow the guest's memory foot print to shrink as seen from the host, it needs to report to the host which pages are not needed anymore by the guest. The kernel has an API for that called Free Page Reporting and since it's built-in it's as easy as starting QEMU like this:

$ qemu-system-x86_64 ... -device virtio-balloon,free-page-reporting=on
After this, you should see the guest memory increasing and then shrinking again after running workloads in it.

However, while this parameter will indeed take care of shrinking the guest's memory usage from the host's perspective when pages are freed, it will not be able to automatically make use of memory that the guest is using for cache. This is an important consideration as a guest is likely to eventually use its entire unused memory for caching, making free-page-reporting=on useless. Read the next section to mitigate this problem.

Using virtio pmem to bypass the guest's page cache
You might want to rely on the host's page cache instead of the guest's in order to allow for more efficient memory usage. Coupled with KSM, this allows you to make your virtual machines quite memory efficient, duplicating only few pages.

One way to achieve this is to use a virtio pmem device. Add this config to your QEMU:

-object memory-backend-file,id=mem1,share,mem-path=./virtio_pmem.img,size=32G
-device virtio-pmem-pci,memdev=mem1,id=nv1
-m 64G,maxmem=96G
whereby virtio_pmem.img is a local file on the host that will serve as our memory backend in side the guest. The -m part is important here: Set the maxmem parameter so that it is regular memory + memory-backend-file size. In this case: 64G + 32G = 96G.

Start the guest with those options. Inside the guest, you will find a new device at /dev/pmem0 which we will need to format with a filesystem such as ext4 (btrfs is not supported):

# mkfs.ext4 /dev/pmem0
mount /dev/pmem0 /mnt -o dax=always
Any files you write into /mnt will then bypass the guest's page cache.

It's also possible to have the whole root filesystem DAX-enabled in this way.

Preparing a Windows guest
Virtio drivers for Windows
Windows does not come with the virtio drivers. The latest and stable versions of the drivers are regularly built by Fedora, details on downloading the drivers are given on virtio-win on GitHub. In the following sections we will mostly use the stable ISO file provided here: virtio-win.iso. Alternatively, use virtio-winAUR.

Block device drivers
New Install of Windows
The drivers need to be loaded during installation, the procedure is to load the ISO image with the virtio drivers in a cdrom device along with the primary disk device and the Windows ISO install media:

$ qemu-system-x86_64 ... \
-drive file=disk_image,index=0,media=disk,if=virtio \
-drive file=windows.iso,index=2,media=cdrom \
-drive file=virtio-win.iso,index=3,media=cdrom \
...
During the installation, at some stage, the Windows installer will ask "Where do you want to install Windows?", it will give a warning that no disks are found. Follow the example instructions below (based on Windows Server 2012 R2 with Update).

Select the option Load Drivers.
Uncheck the box for Hide drivers that are not compatible with this computer's hardware.
Click the browse button and open the CDROM for the virtio iso, usually named "virtio-win-XX".
Now browse to E:\viostor\[your-os]\amd64, select it, and confirm.
You should now see your virtio disk(s) listed here, ready to be selected, formatted and installed to.

Change existing Windows virtual machine to use virtio
Modifying an existing Windows guest for booting from virtio disk requires that the virtio driver is loaded by the guest at boot time. We will therefore need to teach Windows to load the virtio driver at boot time before being able to boot a disk image in virtio mode.

To achieve that, first create a new disk image that will be attached in virtio mode and trigger the search for the driver:

$ qemu-img create -f qcow2 dummy.qcow2 1G
Run the original Windows guest with the boot disk still in IDE mode, the fake disk in virtio mode and the driver ISO image.

$ qemu-system-x86_64 -m 4G -drive file=disk_image,if=ide -drive file=dummy.qcow2,if=virtio -cdrom virtio-win.iso
Windows will detect the fake disk and look for a suitable driver. If it fails, go to Device Manager, locate the SCSI drive with an exclamation mark icon (should be open), click Update driver and select the virtual CD-ROM. Do not navigate to the driver folder within the CD-ROM, simply select the CD-ROM drive and Windows will find the appropriate driver automatically (tested for Windows 7 SP1).

Request Windows to boot in safe mode next time it starts up. This can be done using the msconfig.exe tool in Windows. In safe mode all the drivers will be loaded at boot time including the new virtio driver. Once Windows knows that the virtio driver is required at boot it will memorize it for future boot.

Once instructed to boot in safe mode, you can turn off the virtual machine and launch it again, now with the boot disk attached in virtio mode:

$ qemu-system-x86_64 -m 4G -drive file=disk_image,if=virtio
You should boot in safe mode with virtio driver loaded, you can now return to msconfig.exe disable safe mode boot and restart Windows.

Note: If you encounter the blue screen of death using the if=virtio parameter, it probably means the virtio disk driver is not installed or not loaded at boot time, reboot in safe mode and check your driver configuration.
Network drivers
Using virtio network drivers is a bit easier, simply add the -nic argument.

$ qemu-system-x86_64 -m 4G -drive file=windows_disk_image,if=virtio -nic user,model=virtio-net-pci -cdrom virtio-win.iso
Windows will detect the network adapter and try to find a driver for it. If it fails, go to the Device Manager, locate the network adapter with an exclamation mark icon (should be open), click Update driver and select the virtual CD-ROM. Do not forget to select the checkbox which says to search for directories recursively.

Balloon driver
If you want to track you guest memory state (for example via virsh command dommemstat) or change guest's memory size in runtime (you still will not be able to change memory size, but can limit memory usage via inflating balloon driver) you will need to install guest balloon driver.

For this you will need to go to Device Manager, locate PCI standard RAM Controller in System devices (or unrecognized PCI controller from Other devices) and choose Update driver. In opened window you will need to choose Browse my computer... and select the CD-ROM (and do not forget the Include subdirectories checkbox). Reboot after installation. This will install the driver and you will be able to inflate the balloon (for example via hmp command balloon memory_size, which will cause balloon to take as much memory as possible in order to shrink the guest's available memory size to memory_size). However, you still will not be able to track guest memory state. In order to do this you will need to install Balloon service properly. For that open command line as administrator, go to the CD-ROM, Balloon directory and deeper, depending on your system and architecture. Once you are in amd64 (x86) directory, run blnsrv.exe -i which will do the installation. After that virsh command dommemstat should be outputting all supported values.

Using a virtiofsd share
Before you progress in this section, make sure you followed the section about setting up host file sharing with virtiofsd first.

First, follow the upstream instructions. Once configured, Windows will have the Z: drive mapped automatically with shared directory content.

Your Windows 11 guest system is properly configured if it has:

VirtioFSSService windows service,
WinFsp.Launcher windows service,
VirtIO FS Device driver under "System devices" in Windows "Device Manager".
If the above installed and Z: drive is still not listed, try repairing "Virtio-win-guest-tools" in Windows Add/Remove programs.

Preparing a FreeBSD guest
Install the emulators/virtio-kmod port if you are using FreeBSD 8.3 or later up until 10.0-CURRENT where they are included into the kernel. After installation, add the following to your /boot/loader.conf file:

virtio_load="YES"
virtio_pci_load="YES"
virtio_blk_load="YES"
if_vtnet_load="YES"
virtio_balloon_load="YES"
Then modify your /etc/fstab by doing the following:

# sed -ibak "s/ada/vtbd/g" /etc/fstab
And verify that /etc/fstab is consistent. If anything goes wrong, just boot into a rescue CD and copy /etc/fstab.bak back to /etc/fstab.

QEMU monitor
While QEMU is running, a monitor console is provided in order to provide several ways to interact with the virtual machine running. The QEMU monitor offers interesting capabilities such as obtaining information about the current virtual machine, hotplugging devices, creating snapshots of the current state of the virtual machine, etc. To see the list of all commands, run help or ? in the QEMU monitor console or review the relevant section of the official QEMU documentation.

Accessing the monitor console
Graphical view
When using the std default graphics option, one can access the QEMU monitor by pressing Ctrl+Alt+2 or by clicking View > compatmonitor0 in the QEMU window. To return to the virtual machine graphical view either press Ctrl+Alt+1 or click View > VGA.

However, the standard method of accessing the monitor is not always convenient and does not work in all graphic outputs QEMU supports.

Telnet
To enable telnet, run QEMU with the -monitor telnet:127.0.0.1:port,server,nowait parameter. When the virtual machine is started you will be able to access the monitor via telnet:

$ telnet 127.0.0.1 port
Note: If 127.0.0.1 is specified as the IP to listen it will be only possible to connect to the monitor from the same host QEMU is running on. If connecting from remote hosts is desired, QEMU must be told to listen 0.0.0.0 as follows: -monitor telnet:0.0.0.0:port,server,nowait. Keep in mind that it is recommended to have a firewall configured in this case or make sure your local network is completely trustworthy since this connection is completely unauthenticated and unencrypted.
UNIX socket
Run QEMU with the -monitor unix:socketfile,server,nowait parameter. Then you can connect with either socat, nmap or openbsd-netcat.

For example, if QEMU is run via:

$ qemu-system-x86_64 -monitor unix:/tmp/monitor.sock,server,nowait [...]
It is possible to connect to the monitor with:

$ socat - UNIX-CONNECT:/tmp/monitor.sock
Or with:

$ nc -U /tmp/monitor.sock
Alternatively with nmap:

$ ncat -U /tmp/monitor.sock
TCP
You can expose the monitor over TCP with the argument -monitor tcp:127.0.0.1:port,server,nowait. Then connect with netcat, either openbsd-netcat or gnu-netcat by running:

$ nc 127.0.0.1 port
Note: In order to be able to connect to the tcp socket from other devices other than the same host QEMU is being run on you need to listen to 0.0.0.0 like explained in the telnet case. The same security warnings apply in this case as well.
Standard I/O
It is possible to access the monitor automatically from the same terminal QEMU is being run by running it with the argument -monitor stdio.

Sending keyboard presses to the virtual machine using the monitor console
Some combinations of keys may be difficult to perform on virtual machines due to the host intercepting them instead in some configurations (a notable example is the Ctrl+Alt+F* key combinations, which change the active tty). To avoid this problem, the problematic combination of keys may be sent via the monitor console instead. Switch to the monitor and use the sendkey command to forward the necessary keypresses to the virtual machine. For example:

(qemu) sendkey ctrl-alt-f2
Creating and managing snapshots via the monitor console
Note: This feature will only work when the virtual machine disk image is in qcow2 format. It will not work with raw images.
It is sometimes desirable to save the current state of a virtual machine and having the possibility of reverting the state of the virtual machine to that of a previously saved snapshot at any time. The QEMU monitor console provides the user with the necessary utilities to create snapshots, manage them, and revert the machine state to a saved snapshot.

Use savevm name in order to create a snapshot with the tag name.
Use loadvm name to revert the virtual machine to the state of the snapshot name.
Use delvm name to delete the snapshot tagged as name.
Use info snapshots to see a list of saved snapshots. Snapshots are identified by both an auto-incremented ID number and a text tag (set by the user on snapshot creation).
Running the virtual machine in immutable mode
It is possible to run a virtual machine in a frozen state so that all changes will be discarded when the virtual machine is powered off just by running QEMU with the -snapshot parameter. When the disk image is written by the guest, changes will be saved in a temporary file in /tmp and will be discarded when QEMU halts.

However, if a machine is running in frozen mode it is still possible to save the changes to the disk image if it is afterwards desired by using the monitor console and running the following command:

(qemu) commit all
If snapshots are created when running in frozen mode they will be discarded as soon as QEMU is exited unless changes are explicitly commited to disk, as well.

Pause and power options via the monitor console
Some operations of a physical machine can be emulated by QEMU using some monitor commands:

system_powerdown will send an ACPI shutdown request to the virtual machine. This effect is similar to the power button in a physical machine.
system_reset will reset the virtual machine similarly to a reset button in a physical machine. This operation can cause data loss and file system corruption since the virtual machine is not cleanly restarted.
stop will pause the virtual machine.
cont will resume a virtual machine previously paused.
Taking screenshots of the virtual machine
Screenshots of the virtual machine graphic display can be obtained in the PPM format by running the following command in the monitor console:

(qemu) screendump file.ppm
QEMU machine protocol
The QEMU machine protocol (QMP) is a JSON-based protocol which allows applications to control a QEMU instance. Similarly to the #QEMU monitor it offers ways to interact with a running machine and the JSON protocol allows to do it programmatically. The description of all the QMP commands can be found in qmp-commands.

Start QMP
The usual way to control the guest using the QMP protocol, is to open a TCP socket when launching the machine using the -qmp option. Here it is using for example the TCP port 4444:

$ qemu-system-x86_64 [...] -qmp tcp:localhost:4444,server,nowait
Then one way to communicate with the QMP agent is to use netcat:

nc localhost 4444
{"QMP": {"version": {"qemu": {"micro": 0, "minor": 1, "major": 3}, "package": ""}, "capabilities": []} } 
At this stage, the only command that can be recognized is qmp_capabilities, so that QMP enters into command mode. Type:

{"execute": "qmp_capabilities"}
Now, QMP is ready to receive commands, to retrieve the list of recognized commands, use:

{"execute": "query-commands"}
Live merging of child image into parent image
It is possible to merge a running snapshot into its parent by issuing a block-commit command. In its simplest form the following line will commit the child into its parent:

{"execute": "block-commit", "arguments": {"device": "devicename"}}
Upon reception of this command, the handler looks for the base image and converts it from read only to read write mode and then runs the commit job.

Once the block-commit operation has completed, the event BLOCK_JOB_READY will be emitted, signalling that the synchronization has finished. The job can then be gracefully completed by issuing the command block-job-complete:

{"execute": "block-job-complete", "arguments": {"device": "devicename"}}
Until such a command is issued, the commit operation remains active. After successful completion, the base image remains in read write mode and becomes the new active layer. On the other hand, the child image becomes invalid and it is the responsibility of the user to clean it up.

Tip: The list of device and their names can be retrieved by executing the command query-block and parsing the results. The device name is in the device field, for example ide0-hd0 for the hard disk in this example:
{"execute": "query-block"}
{"return": [{"io-status": "ok", "device": "ide0-hd0", "locked": false, "removable": false, "inserted": {"iops_rd": 0, "detect_zeroes": "off", "image": {"backing-image": {"virtual-size": 27074281472, "filename": "parent.qcow2", ... } 
Live creation of a new snapshot
To create a new snapshot out of a running image, run the command:

{"execute": "blockdev-snapshot-sync", "arguments": {"device": "devicename","snapshot-file": "new_snapshot_name.qcow2"}}
This creates an overlay file named new_snapshot_name.qcow2 which then becomes the new active layer.

Tips and tricks
Improve virtual machine performance
There are a number of techniques that you can use to improve the performance of the virtual machine. For example:

Apply #Enabling KVM for full virtualization.
Use the -cpu host option to make QEMU emulate the host's exact CPU rather than a more generic CPU.
Especially for Windows guests, enable Hyper-V enlightenments: -cpu host,hv_relaxed,hv_spinlocks=0x1fff,hv_vapic,hv_time. See the QEMU documentation for more information and flags.
multiple cores can be assigned to the guest using the -smp cores=x,threads=y,sockets=1,maxcpus=z option. The threads parameter is used to assign SMT cores. Leaving a physical core for QEMU, the hypervisor and the host system to operate unimpeded is highly beneficial.
Make sure you have assigned the virtual machine enough memory. By default, QEMU only assigns 128 MiB of memory to each virtual machine. Use the -m option to assign more memory. For example, -m 1024 runs a virtual machine with 1024 MiB of memory.
If supported by drivers in the guest operating system, use virtio for network and/or block devices, see #Using virtio drivers.
Use TAP devices instead of user-mode networking, see #Tap networking with QEMU.
If the guest OS is doing heavy writing to its disk, you may benefit from certain mount options on the host's file system. For example, you can mount an ext4 file system with the option barrier=0. You should read the documentation for any options that you change because sometimes performance-enhancing options for file systems come at the cost of data integrity.
If you have a raw disk or partition, you may want to disable the cache:
$ qemu-system-x86_64 -drive file=/dev/disk,if=virtio,cache=none
Use the native Linux AIO:
$ qemu-system-x86_64 -drive file=disk_image,if=virtio,aio=native,cache.direct=on
If you are running multiple virtual machines concurrently that all have the same operating system installed, you can save memory by enabling kernel same-page merging. See #Enabling KSM.
In some cases, memory can be reclaimed from running virtual machines by running a memory ballooning driver in the guest operating system. See #Memory ballooning.
It is possible to use a emulation layer for an ICH-9 AHCI controller (although it may be unstable). The AHCI emulation supports NCQ, so multiple read or write requests can be outstanding at the same time:
$ qemu-system-x86_64 -drive id=disk,file=disk_image,if=none -device ich9-ahci,id=ahci -device ide-drive,drive=disk,bus=ahci.0
See https://www.linux-kvm.org/page/Tuning_KVM for more information.

Using any real partition as the single primary partition of a hard disk image
Sometimes, you may wish to use one of your system partitions from within QEMU. Using a raw partition for a virtual machine will improve performance, as the read and write operations do not go through the file system layer on the physical host. Such a partition also provides a way to share data between the host and guest.

In Arch Linux, device files for raw partitions are, by default, owned by root and the disk group. If you would like to have a non-root user be able to read and write to a raw partition, you must either change the owner of the partition's device file to that user, add that user to the disk group, or use ACL for more fine-grained access control.

Warning:
Although it is possible, it is not recommended to allow virtual machines to alter critical data on the host system, such as the root partition.
You must not mount a file system on a partition read-write on both the host and the guest at the same time. Otherwise, data corruption will result.
After doing so, you can attach the partition to a QEMU virtual machine as a virtual disk.

However, things are a little more complicated if you want to have the entire virtual machine contained in a partition. In that case, there would be no disk image file to actually boot the virtual machine since you cannot install a boot loader to a partition that is itself formatted as a file system and not as a partitioned device with an MBR. Such a virtual machine can be booted either by: #Specifying kernel and initrd manually, #Simulating a virtual disk with MBR, #Using the device-mapper, #Using a linear RAID or #Using a Network Block Device.

Specifying kernel and initrd manually
QEMU supports loading Linux kernels and init ramdisks directly, thereby circumventing boot loaders such as GRUB. It then can be launched with the physical partition containing the root file system as the virtual disk, which will not appear to be partitioned. This is done by issuing a command similar to the following:

Note: In this example, it is the host's images that are being used, not the guest's. If you wish to use the guest's images, either mount /dev/sda3 read-only (to protect the file system from the host) and specify the /full/path/to/images or use some kexec hackery in the guest to reload the guest's kernel (extends boot time).
$ qemu-system-x86_64 -kernel /boot/vmlinuz-linux -initrd /boot/initramfs-linux.img -append root=/dev/sda /dev/sda3
In the above example, the physical partition being used for the guest's root file system is /dev/sda3 on the host, but it shows up as /dev/sda on the guest.

You may, of course, specify any kernel and initrd that you want, and not just the ones that come with Arch Linux.

When there are multiple kernel parameters to be passed to the -append option, they need to be quoted using single or double quotes. For example:

... -append 'root=/dev/sda1 console=ttyS0'
Simulating a virtual disk with MBR
A more complicated way to have a virtual machine use a physical partition, while keeping that partition formatted as a file system and not just having the guest partition the partition as if it were a disk, is to simulate an MBR for it so that it can boot using a boot loader such as GRUB.

For the following, suppose you have a plain, unmounted /dev/hdaN partition with some file system on it you wish to make part of a QEMU disk image. The trick is to dynamically prepend a master boot record (MBR) to the real partition you wish to embed in a QEMU raw disk image. More generally, the partition can be any part of a larger simulated disk, in particular a block device that simulates the original physical disk but only exposes /dev/hdaN to the virtual machine.

A virtual disk of this type can be represented by a VMDK file that contains references to (a copy of) the MBR and the partition, but QEMU does not support this VMDK format. For instance, a virtual disk created by

$ VBoxManage internalcommands createrawvmdk -filename /path/to/file.vmdk -rawdisk /dev/hda
will be rejected by QEMU with the error message

Unsupported image type 'partitionedDevice'
Note that VBoxManage creates two files, file.vmdk and file-pt.vmdk, the latter being a copy of the MBR, to which the text file file.vmdk points. Read operations outside the target partition or the MBR would give zeros, while written data would be discarded.

Using the device-mapper
A method that is similar to the use of a VMDK descriptor file uses the device-mapper to prepend a loop device attached to the MBR file to the target partition. In case we do not need our virtual disk to have the same size as the original, we first create a file to hold the MBR:

$ dd if=/dev/zero of=/path/to/mbr count=2048
Here, a 1 MiB (2048 * 512 bytes) file is created in accordance with partition alignment policies used by modern disk partitioning tools. For compatibility with older partitioning software, 63 sectors instead of 2048 might be required. The MBR only needs a single 512 bytes block, the additional free space can be used for a BIOS boot partition and, in the case of a hybrid partitioning scheme, for a GUID Partition Table. Then, we attach a loop device to the MBR file:

# losetup --show -f /path/to/mbr
/dev/loop0
In this example, the resulting device is /dev/loop0. The device mapper is now used to join the MBR and the partition:

# echo "0 2048 linear /dev/loop0 0
2048 `blockdev --getsz /dev/hdaN` linear /dev/hdaN 0" | dmsetup create qemu
The resulting /dev/mapper/qemu is what we will use as a QEMU raw disk image. Additional steps are required to create a partition table (see the section that describes the use of a linear RAID for an example) and boot loader code on the virtual disk (which will be stored in /path/to/mbr).

The following setup is an example where the position of /dev/hdaN on the virtual disk is to be the same as on the physical disk and the rest of the disk is hidden, except for the MBR, which is provided as a copy:

# dd if=/dev/hda count=1 of=/path/to/mbr
# loop=`losetup --show -f /path/to/mbr`
# start=`blockdev --report /dev/hdaN | tail -1 | awk '{print $5}'`
# size=`blockdev --getsz /dev/hdaN`
# disksize=`blockdev --getsz /dev/hda`
# echo "0 1 linear $loop 0
1 $((start-1)) zero
$start $size linear /dev/hdaN 0
$((start+size)) $((disksize-start-size)) zero" | dmsetup create qemu
The table provided as standard input to dmsetup has a similar format as the table in a VMDK descriptor file produced by VBoxManage and can alternatively be loaded from a file with dmsetup create qemu --table table_file. To the virtual machine, only /dev/hdaN is accessible, while the rest of the hard disk reads as zeros and discards written data, except for the first sector. We can print the table for /dev/mapper/qemu with dmsetup table qemu (use udevadm info -rq name /sys/dev/block/major:minor to translate major:minor to the corresponding /dev/blockdevice name). Use dmsetup remove qemu and losetup -d $loop to delete the created devices.

A situation where this example would be useful is an existing Windows XP installation in a multi-boot configuration and maybe a hybrid partitioning scheme (on the physical hardware, Windows XP could be the only operating system that uses the MBR partition table, while more modern operating systems installed on the same computer could use the GUID Partition Table). Windows XP supports hardware profiles, so that that the same installation can be used with different hardware configurations alternatingly (in this case bare metal vs. virtual) with Windows needing to install drivers for newly detected hardware only once for every profile. Note that in this example the boot loader code in the copied MBR needs to be updated to directly load Windows XP from /dev/hdaN instead of trying to start the multi-boot capable boot loader (like GRUB) present in the original system. Alternatively, a copy of the boot partition containing the boot loader installation can be included in the virtual disk the same way as the MBR.

Using a linear RAID
You can also do this using software RAID in linear mode (you need the linear.ko kernel driver) and a loopback device:

First, you create some small file to hold the MBR:

$ dd if=/dev/zero of=/path/to/mbr count=32
Here, a 16 KiB (32 * 512 bytes) file is created. It is important not to make it too small (even if the MBR only needs a single 512 bytes block), since the smaller it will be, the smaller the chunk size of the software RAID device will have to be, which could have an impact on performance. Then, you setup a loopback device to the MBR file:

# losetup -f /path/to/mbr
Let us assume the resulting device is /dev/loop0, because we would not already have been using other loopbacks. Next step is to create the "merged" MBR + /dev/hdaN disk image using software RAID:

# modprobe linear
# mdadm --build --verbose /dev/md0 --chunk=16 --level=linear --raid-devices=2 /dev/loop0 /dev/hdaN
The resulting /dev/md0 is what you will use as a QEMU raw disk image (do not forget to set the permissions so that the emulator can access it). The last (and somewhat tricky) step is to set the disk configuration (disk geometry and partitions table) so that the primary partition start point in the MBR matches the one of /dev/hdaN inside /dev/md0 (an offset of exactly 16 * 512 = 16384 bytes in this example). Do this using fdisk on the host machine, not in the emulator: the default raw disc detection routine from QEMU often results in non-kibibyte-roundable offsets (such as 31.5 KiB, as in the previous section) that cannot be managed by the software RAID code. Hence, from the host:

# fdisk /dev/md0
Press X to enter the expert menu. Set number of 's'ectors per track so that the size of one cylinder matches the size of your MBR file. For two heads and a sector size of 512, the number of sectors per track should be 16, so we get cylinders of size 2x16x512=16k.

Now, press R to return to the main menu.

Press P and check that the cylinder size is now 16k.

Now, create a single primary partition corresponding to /dev/hdaN. It should start at cylinder 2 and end at the end of the disk (note that the number of cylinders now differs from what it was when you entered fdisk.

Finally, 'w'rite the result to the file: you are done. You now have a partition you can mount directly from your host, as well as part of a QEMU disk image:

$ qemu-system-x86_64 -hdc /dev/md0 [...]
You can, of course, safely set any boot loader on this disk image using QEMU, provided the original /dev/hdaN partition contains the necessary tools.

Using a Network Block Device
With Network Block Device, Linux can use a remote server as one of its block device. You may use nbd-server (from the nbd package) to create an MBR wrapper for QEMU.

Assuming you have already set up your MBR wrapper file like above, rename it to wrapper.img.0. Then create a symbolic link named wrapper.img.1 in the same directory, pointing to your partition. Then put the following script in the same directory:

#!/bin/sh
dir="$(realpath "$(dirname "$0")")"
cat >wrapper.conf <<EOF
[generic]
allowlist = true
listenaddr = 127.713705
port = 10809

[wrap]
exportname = $dir/wrapper.img
multifile = true
EOF

nbd-server \
    -C wrapper.conf \
    -p wrapper.pid \
    "$@"
The .0 and .1 suffixes are essential; the rest can be changed. After running the above script (which you may need to do as root to make sure nbd-server is able to access the partition), you can launch QEMU with:

qemu-system-x86_64 -drive file=nbd:127.713705:10809:exportname=wrap [...]
Starting QEMU virtual machines on boot
With libvirt
If a virtual machine is set up with libvirt, it can be configured with virsh autostart or through the virt-manager GUI to start at host boot by going to the Boot Options for the virtual machine and selecting "Start virtual machine on host boot up".

With systemd service
To run QEMU virtual machines on boot, you can use following systemd unit and config.

/etc/systemd/system/qemu@.service
[Unit]
Description=QEMU virtual machine

[Service]
Environment="haltcmd=kill -INT $MAINPID"
EnvironmentFile=/etc/conf.d/qemu.d/%i
ExecStart=/usr/bin/qemu-system-x86_64 -name %i -enable-kvm -m 512 -nographic $args
ExecStop=/usr/bin/bash -c ${haltcmd}
ExecStop=/usr/bin/bash -c 'while nc localhost 7100; do sleep 1; done'

[Install]
WantedBy=multi-user.target
Note: This service will wait for the console port to be released, which means that the virtual machine has been shutdown, to graciously end.
Then create per-VM configuration files, named /etc/conf.d/qemu.d/vm_name, with the variables args and haltcmd set. Example configs:

/etc/conf.d/qemu.d/one
args="-hda /dev/vg0/vm1 -serial telnet:localhost:7000,server,nowait,nodelay \
 -monitor telnet:localhost:7100,server,nowait,nodelay -vnc :0"

haltcmd="echo 'system_powerdown' | nc localhost 7100" # or netcat/ncat
/etc/conf.d/qemu.d/two
args="-hda /srv/kvm/vm2 -serial telnet:localhost:7001,server,nowait,nodelay -vnc :1"

haltcmd="ssh powermanager@vm2 sudo poweroff"
The description of the variables is the following:

args - QEMU command line arguments to be used.
haltcmd - Command to shut down a virtual machine safely. In the first example, the QEMU monitor is exposed via telnet using -monitor telnet:.. and the virtual machines are powered off via ACPI by sending system_powerdown to monitor with the nc command. In the other example, SSH is used.
To set which virtual machines will start on boot-up, enable the qemu@vm_name.service systemd unit.

Mouse integration
To prevent the mouse from being grabbed when clicking on the guest operating system's window, add the options -usb -device usb-tablet. This means QEMU is able to report the mouse position without having to grab the mouse. This also overrides PS/2 mouse emulation when activated. For example:

$ qemu-system-x86_64 -hda disk_image -m 512 -usb -device usb-tablet
If that does not work, try using -vga qxl parameter, also look at the instructions QEMU/Troubleshooting#Mouse cursor is jittery or erratic.

Pass-through host USB device
It is possible to access the physical device connected to a USB port of the host from the guest. The first step is to identify where the device is connected, this can be found running the lsusb command. For example:

$ lsusb
...
Bus 003 Device 007: ID 0781:5406 SanDisk Corp. Cruzer Micro U3
The outputs in bold above will be useful to identify respectively the host_bus and host_addr or the vendor_id and product_id.

In qemu, the idea is to emulate an EHCI (USB 2) or XHCI (USB 1.1 USB 2 USB 3) controller with the option -device usb-ehci,id=ehci or -device qemu-xhci,id=xhci respectively and then attach the physical device to it with the option -device usb-host,... We will consider that controller_id is either ehci or xhci for the rest of this section.

Then, there are two ways to connect to the USB of the host with qemu:

Identify the device and connect to it on any bus and address it is attached to on the host, the generic syntax is:
-device usb-host,bus=controller_id.0,vendorid=0xvendor_id,productid=0xproduct_id
Applied to the device used in the example above, it becomes:
-device usb-ehci,id=ehci -device usb-host,bus=ehci.0,vendorid=0x0781,productid=0x5406
One can also add the ...,port=port_number setting to the previous option to specify in which physical port of the virtual controller the device should be attached, useful in the case one wants to add multiple USB devices to the virtual machine. Another option is to use the new hostdevice property of usb-host which is available since QEMU 5.1.0, the syntax is:
-device qemu-xhci,id=xhci -device usb-host,hostdevice=/dev/bus/usb/003/007
Attach whatever is connected to a given USB bus and address, the syntax is:
-device usb-host,bus=controller_id.0,hostbus=host_bus,host_addr=host_addr
Applied to the bus and the address in the example above, it becomes:
-device usb-ehci,id=ehci -device usb-host,bus=ehci.0,hostbus=3,hostaddr=7
See QEMU/USB emulation for more information.

Note: If you encounter permission errors when running QEMU, see udev#About udev rules for information on how to set permissions of the device.
USB redirection with SPICE
When using #SPICE it is possible to redirect USB devices from the client to the virtual machine without needing to specify them in the QEMU command. It is possible to configure the number of USB slots available for redirected devices (the number of slots will determine the maximum number of devices which can be redirected simultaneously). The main advantages of using SPICE for redirection compared to the previously-mentioned -usbdevice method is the possibility of hot-swapping USB devices after the virtual machine has started, without needing to halt it in order to remove USB devices from the redirection or adding new ones. This method of USB redirection also allows us to redirect USB devices over the network, from the client to the server. In summary, it is the most flexible method of using USB devices in a QEMU virtual machine.

We need to add one EHCI/UHCI controller per available USB redirection slot desired as well as one SPICE redirection channel per slot. For example, adding the following arguments to the QEMU command you use for starting the virtual machine in SPICE mode will start the virtual machine with three available USB slots for redirection:

-device ich9-usb-ehci1,id=usb \
-device ich9-usb-uhci1,masterbus=usb.0,firstport=0,multifunction=on \
-device ich9-usb-uhci2,masterbus=usb.0,firstport=2 \
-device ich9-usb-uhci3,masterbus=usb.0,firstport=4 \
-chardev spicevmc,name=usbredir,id=usbredirchardev1 -device usb-redir,chardev=usbredirchardev1,id=usbredirdev1 \
-chardev spicevmc,name=usbredir,id=usbredirchardev2 -device usb-redir,chardev=usbredirchardev2,id=usbredirdev2 \
-chardev spicevmc,name=usbredir,id=usbredirchardev3 -device usb-redir,chardev=usbredirchardev3,id=usbredirdev3
See SPICE/usbredir for more information.

Both spicy from spice-gtk (Input > Select USB Devices for redirection) and remote-viewer from virt-viewer (File > USB device selection) support this feature. Please make sure that you have installed the necessary SPICE Guest Tools on the virtual machine for this functionality to work as expected (see the #SPICE section for more information).

Warning: Keep in mind that when a USB device is redirected from the client, it will not be usable from the client operating system itself until the redirection is stopped. It is specially important to never redirect the input devices (namely mouse and keyboard), since it will be then difficult to access the SPICE client menus to revert the situation, because the client will not respond to the input devices after being redirected to the virtual machine.
Automatic USB forwarding with udev
Normally, forwarded devices must be available at the boot time of the virtual machine to be forwarded. If that device is disconnected, it will not be forwarded anymore.

You can use udev rules to automatically attach a device when it comes online. Create a hostdev entry somewhere on disk. chown it to root to prevent other users modifying it.

/usr/local/hostdev-mydevice.xml
<hostdev mode='subsystem' type='usb'>
  <source>
    <vendor id='0x03f0'/>
    <product id='0x4217'/>
  </source>
</hostdev>
Then create a udev rule which will attach/detach the device:

/usr/lib/udev/rules.d/90-libvirt-mydevice
ACTION=="add", \
    SUBSYSTEM=="usb", \
    ENV{ID_VENDOR_ID}=="03f0", \
    ENV{ID_MODEL_ID}=="4217", \
    RUN+="/usr/bin/virsh attach-device GUESTNAME /usr/local/hostdev-mydevice.xml"
ACTION=="remove", \
    SUBSYSTEM=="usb", \
    ENV{ID_VENDOR_ID}=="03f0", \
    ENV{ID_MODEL_ID}=="4217", \
    RUN+="/usr/bin/virsh detach-device GUESTNAME /usr/local/hostdev-mydevice.xml"
Source and further reading.

Enabling KSM
Kernel Samepage Merging (KSM) is a feature of the Linux kernel that allows for an application to register with the kernel to have its pages merged with other processes that also register to have their pages merged. The KSM mechanism allows for guest virtual machines to share pages with each other. In an environment where many of the guest operating systems are similar, this can result in significant memory savings.

Note: Although KSM may reduce memory usage, it may increase CPU usage. Also note some security issues may occur, see Wikipedia:Kernel same-page merging.
To enable KSM:

# echo 1 > /sys/kernel/mm/ksm/run
To make it permanent, use systemd's temporary files:

/etc/tmpfiles.d/ksm.conf
w /sys/kernel/mm/ksm/run - - - - 1
If KSM is running, and there are pages to be merged (i.e. at least two similar virtual machines are running), then /sys/kernel/mm/ksm/pages_shared should be non-zero. See https://docs.kernel.org/admin-guide/mm/ksm.html for more information.

Tip: An easy way to see how well KSM is performing is to simply print the contents of all the files in that directory:
$ grep -r . /sys/kernel/mm/ksm/
Multi-monitor support
The Linux QXL driver supports four heads (virtual screens) by default. This can be changed via the qxl.heads=N kernel parameter.

The default VGA memory size for QXL devices is 16M (VRAM size is 64M). This is not sufficient if you would like to enable two 1920x1200 monitors since that requires 2 × 1920 × 4 (color depth) × 1200 = 17.6 MiB VGA memory. This can be changed by replacing -vga qxl by -vga none -device qxl-vga,vgamem_mb=32. If you ever increase vgamem_mb beyond 64M, then you also have to increase the vram_size_mb option.

Custom display resolution
A custom display resolution can be set with -device VGA,edid=on,xres=1280,yres=720 (see EDID and display resolution).

Copy and paste
SPICE
One way to share the clipboard between the host and the guest is to enable the SPICE remote desktop protocol and access the client with a SPICE client. One needs to follow the steps described in #SPICE. A guest run this way will support copy paste with the host.

qemu-vdagent
QEMU provides its own implementation of the spice vdagent chardev called qemu-vdagent. It interfaces with the spice-vdagent guest service and allows the guest and host share a clipboard.

To access this shared clipboard with QEMU's GTK display, you will need to compile QEMU from source with the --enable-gtk-clipboard configure parameter. It is sufficient to replace the installed qemu-ui-gtk package.

Note:
Feature request FS#79716 submitted to enable the functionality in the official package.
The shared clipboard in qemu-ui-gtk has been pushed back to experimental as it can freeze guests under certain circumstances. A fix has been proposed to solve the issue upstream.
Add the following QEMU command line arguments:

-device virtio-serial,packed=on,ioeventfd=on
-device virtserialport,name=com.redhat.spice.0,chardev=vdagent0
-chardev qemu-vdagent,id=vdagent0,name=vdagent,clipboard=on,mouse=off
These arguments are also valid if converted to libvirt form.

Note: While the spicevmc chardev will start the spice-vdagent service of the guest automatically, the qemu-vdagent chardev may not.
On linux guests, you may start the spice-vdagent.service user unit manually. On Windows guests, set the spice-agent startup type to automatic.

Windows-specific notes
QEMU can run any version of Windows from Windows 95 through Windows 11.

It is possible to run Windows PE in QEMU.

Fast startup
Note: An administrator account is required to change power settings.
For Windows 8 (or later) guests it is better to disable "Turn on fast startup (recommended)" from the Power Options of the Control Panel as explained in the following forum page, as it causes the guest to hang during every other boot.

Fast Startup may also need to be disabled for changes to the -smp option to be properly applied.

Remote Desktop Protocol
If you use a MS Windows guest, you might want to use RDP to connect to your guest virtual machine. If you are using a VLAN or are not in the same network as the guest, use:

$ qemu-system-x86_64 -nographic -nic user,hostfwd=tcp::5555-:3389
Then connect with either rdesktop or freerdp to the guest. For example:

$ xfreerdp -g 2048x1152 localhost:5555 -z -x lan
Clone Linux system installed on physical equipment
Linux system installed on physical equipment can be cloned for running on a QEMU virtual machine. See Clone Linux system from hardware for QEMU virtual machine

Chrooting into arm/arm64 environment from x86_64
Sometimes it is easier to work directly on a disk image instead of the real ARM based device. This can be achieved by mounting an SD card/storage containing the root partition and chrooting into it.

Another use case for an ARM chroot is building ARM packages on an x86_64 machine. Here, the chroot environment can be created from an image tarball from Arch Linux ARM - see [5] for a detailed description of this approach.

Either way, from the chroot it should be possible to run pacman and install more packages, compile large libraries etc. Since the executables are for the ARM architecture, the translation to x86 needs to be performed by QEMU.

Install qemu-user-static on the x86_64 machine/host, and qemu-user-static-binfmt to register the qemu binaries to binfmt service.

qemu-user-static is used to allow the execution of compiled programs from other architectures. This is similar to what is provided by qemu-emulators-full, but the "static" variant is required for chroot. Examples:

qemu-arm-static path_to_sdcard/usr/bin/ls
qemu-aarch64-static path_to_sdcard/usr/bin/ls
These two lines execute the ls command compiled for 32-bit ARM and 64-bit ARM respectively. Note that this will not work without chrooting, because it will look for libraries not present in the host system.

qemu-user-static-binfmt allows automatically prefixing the ARM executable with qemu-arm-static or qemu-aarch64-static.

Make sure that the ARM executable support is active:

$ ls /proc/sys/fs/binfmt_misc
qemu-aarch64  qemu-arm	  qemu-cris  qemu-microblaze  qemu-mipsel  qemu-ppc64	    qemu-riscv64  qemu-sh4    qemu-sparc	qemu-sparc64  status
qemu-alpha    qemu-armeb  qemu-m68k  qemu-mips	      qemu-ppc	   qemu-ppc64abi32  qemu-s390x	  qemu-sh4eb  qemu-sparc32plus	register
Each executable must be listed.

If it is not active, restart systemd-binfmt.service.

Mount the SD card to /mnt/sdcard (the device name may be different).

# mount --mkdir /dev/mmcblk0p2 /mnt/sdcard
Mount boot partition if needed (again, use the suitable device name):

# mount /dev/mmcblk0p1 /mnt/sdcard/boot
Finally chroot into the SD card root as described in Change root#Using chroot:

# chroot /mnt/sdcard /bin/bash
Alternatively, you can use arch-chroot from arch-install-scripts, as it will provide an easier way to get network support:

# arch-chroot /mnt/sdcard /bin/bash
You can also use systemd-nspawn to chroot into the ARM environment:

# systemd-nspawn -D /mnt/sdcard -M myARMMachine --bind-ro=/etc/resolv.conf
--bind-ro=/etc/resolv.conf is optional and gives a working network DNS inside the chroot

sudo in chroot
If you install sudo in the chroot and receive the following error when trying to use it:

sudo: effective uid is not 0, is /usr/bin/sudo on a file system with the 'nosuid' option set or an NFS file system without root privileges?
then you may need to modify the binfmt flags, for example for aarch64:

# cp /usr/lib/binfmt.d/qemu-aarch64-static.conf /etc/binfmt.d/
# vi /etc/binfmt.d/qemu-aarch64-static.conf
and add a C at the end of this file:

:qemu-aarch64:M::\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\xb7\x00:\xff\xff\xff\xff\xff\xff\xff\x00\xff\xff\xff\xff\xff\xff\xff\xff\xfe\xff\xff\xff:/usr/bin/qemu-aarch64-static:FPC
Then restart systemd-binfmt.service and check that the changes have taken effect (note the C on the flags line):

# cat /proc/sys/fs/binfmt_misc/qemu-aarch64
enabled
interpreter /usr/bin/qemu-aarch64-static
flags: POCF
offset 0
magic 7f454c460201010000000000000000000200b700
mask ffffffffffffff00fffffffffffffffffeffffff
See the "flags" section of the kernel binfmt documentation for more information.

Not grabbing mouse input
This article or section needs language, wiki syntax or style improvements. See Help:Style for reference.

Reason: It is not explained what the option actually does. Is it causing or avoiding the side effect? (Discuss in Talk:QEMU)
Tablet mode has side effect of not grabbing mouse input in QEMU window:

-usb -device usb-tablet
It works with several -vga backends one of which is virtio.