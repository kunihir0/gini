LVM

5 languages
Page
Discussion
Read
View source
View history

Tools
Related articles
Install Arch Linux on LVM
LVM on software RAID
dm-crypt/Encrypting an entire system#LVM on LUKS
dm-crypt/Encrypting an entire system#LUKS on LVM
Resizing LVM-on-LUKS
Create root filesystem snapshots with LVM
From Wikipedia:Logical Volume Manager (Linux):

Logical Volume Manager (LVM) is a device mapper framework that provides logical volume management for the Linux kernel.
Background
LVM building blocks
Logical Volume Management utilizes the kernel's device-mapper feature to provide a system of partitions independent of underlying disk layout. With LVM you abstract your storage and have "virtual partitions", making extending/shrinking easier (subject to potential filesystem limitations).

Virtual partitions allow addition and removal without worry of whether you have enough contiguous space on a particular disk, getting caught up fdisking a disk in use (and wondering whether the kernel is using the old or new partition table), or, having to move other partitions out of the way.

Basic building blocks of LVM:

Physical volume (PV)
Unix block device node, usable for storage by LVM. Examples: a hard disk, an MBR or GPT partition, a loopback file, a device mapper device (e.g. dm-crypt). It hosts an LVM header.
Volume group (VG)
Group of PVs that serves as a container for LVs. PEs are allocated from a VG for a LV.
Logical volume (LV)
"Virtual/logical partition" that resides in a VG and is composed of PEs. LVs are Unix block devices analogous to physical partitions, e.g. they can be directly formatted with a file system.
Physical extent (PE)
The smallest contiguous extent (default 4 MiB) in the PV that can be assigned to a LV. Think of PEs as parts of PVs that can be allocated to any LV.
Example:

Physical disks

  Disk1 (/dev/sda):
    ┌──────────────────────────────────────┬─────────────────────────────────────┐
    │ Partition1  50 GiB (Physical volume) │ Partition2 80 GiB (Physical volume) │
    │ /dev/sda1                            │ /dev/sda2                           │
    └──────────────────────────────────────┴─────────────────────────────────────┘

  Disk2 (/dev/sdb):
    ┌──────────────────────────────────────┐
    │ Partition1 120 GiB (Physical volume) │
    │ /dev/sdb1                            │
    └──────────────────────────────────────┘
LVM logical volumes

  Volume Group1 (/dev/MyVolGroup/ = /dev/sda1 + /dev/sda2 + /dev/sdb1):
    ┌─────────────────────────┬─────────────────────────┬──────────────────────────┐
    │ Logical volume1 15 GiB  │ Logical volume2 35 GiB  │ Logical volume3 200 GiB  │
    │ /dev/MyVolGroup/rootvol │ /dev/MyVolGroup/homevol │ /dev/MyVolGroup/mediavol │
    └─────────────────────────┴─────────────────────────┴──────────────────────────┘
Note: Logical volumes are accessible at both /dev/VolumeGroupName/LogicalVolumeName and /dev/mapper/VolumeGroupName-LogicalVolumeName. However, lvm(8) § VALID NAMES recommends the former format for "software and scripts" (e.g. fstab) since the latter is intended for "internal use" and subject to possible "change between releases and distributions".
Advantages
LVM gives you more flexibility than just using normal hard drive partitions:

Use any number of disks as one big disk.
Have logical volumes stretched over several disks (RAID, mirroring, striping which offer advantages such as additional resilience and performance [1]).
Create small logical volumes and resize them "dynamically" as they get filled up.
Resize logical volumes regardless of their order on disk. It does not depend on the position of the LV within VG, there is no need to ensure surrounding available space.
Resize/create/delete logical and physical volumes online. File systems on them still need to be resized, but some (such as Ext4 and Btrfs) support online resizing.
Online/live migration of LV (or segments) being used by services to different disks without having to restart services.
Snapshots allow you to backup a frozen copy of the file system, while keeping service downtime to a minimum and easily merge the snapshot into the original volume later.
Support for unlocking separate volumes without having to enter a key multiple times on boot (make LVM on top of LUKS).
Built-in support for caching of frequently used data (lvmcache(7)).
Disadvantages
Additional steps in setting up the system (may require changes to mkinitcpio configuration), more complicated. Requires (multiple) daemons to constantly run.
If dual-booting, note that Windows does not support LVM; you will be unable to access any LVM partitions from Windows. 3rd Party software may allow to mount certain kinds of LVM setups. [2]
If your physical volumes are not on a RAID-1, RAID-5 or RAID-6 losing one disk can lose one or more logical volumes if you span (or extend) your logical volumes across multiple non-redundant disks.
It is not always easy to shrink the space used by the logical volume manager, meaning the physical volumes used for the logical volumes. If the physical extents are scattered across the physical volume until the end you might need to inspect the segments and move them (potentially to another physical device) or the same device with custom allocation settings (e.g. --alloc anywhere). If you want to dual-boot with other operating systems (e.g. with Microsoft Windows), the only space left on the device for Microsoft Windows is the space not used by LVM / not used as physical volume.
Potentially worse performance than using plain partitions. [3]
May not work well with all file systems, especially those that are designed to be (multi-)device aware. For example, Btrfs offers some of the same functionality (multi device support, (sub)volumes, snapshots and RAID) which could clash (read further about issues with LVM snapshots with Btrfs).
Installation
Make sure the lvm2 package is installed.

If you have LVM volumes not activated via the initramfs, enable lvm2-monitor.service, which is provided by the lvm2 package.

Volume operations
Physical volumes
Creating
To create a PV on /dev/sda1, run:

# pvcreate /dev/sda1
You can check the PV is created using the following command:

# pvs
Growing
After extending or prior to reducing the size of a device that has a physical volume on it, you need to grow or shrink the PV using pvresize(8).

To expand the PV on /dev/sda1 after enlarging the partition, run:

# pvresize /dev/sda1
This will automatically detect the new size of the device and extend the PV to its maximum.

Note: This command can be done while the volume is online.
Shrinking
To shrink a physical volume prior to reducing its underlying device, add the --setphysicalvolumesize size parameters to the command, e.g.:

# pvresize --setphysicalvolumesize 40G /dev/sda1
The above command may leave you with this error:

/dev/sda1: cannot resize to 25599 extents as later ones are allocated.
0 physical volume(s) resized / 1 physical volume(s) not resized
Indeed pvresize will refuse to shrink a PV if it has allocated extents after where its new end would be. One needs to run pvmove beforehand to relocate these elsewhere in the volume group if there is sufficient free space.

Move physical extents
Before freeing up physical extents at the end of the volume, one must run pvdisplay -v -m to see them. An alternative way to view segments in a tabular form is pvs --segments -v.

In the below example, there is one physical volume on /dev/sdd1, one volume group vg1 and one logical volume backup.

# pvdisplay -v -m
    Finding all volume groups.
    Using physical volume(s) on command line.
  --- Physical volume ---
  PV Name               /dev/sdd1
  VG Name               vg1
  PV Size               1.52 TiB / not usable 1.97 MiB
  Allocatable           yes 
  PE Size               4.00 MiB
  Total PE              399669
  Free PE               153600
  Allocated PE          246069
  PV UUID               MR9J0X-zQB4-wi3k-EnaV-5ksf-hN1P-Jkm5mW
   
  --- Physical Segments ---
  Physical extent 0 to 153600:
    FREE
  Physical extent 153601 to 307199:
    Logical volume	/dev/vg1/backup
    Logical extents	1 to 153599
  Physical extent 307200 to 307200:
    FREE
  Physical extent 307201 to 399668:
    Logical volume	/dev/vg1/backup
    Logical extents	153601 to 246068
One can observe FREE space are split across the volume. To shrink the physical volume, we must first move all used segments to the beginning.

Here, the first free segment is from 0 to 153600 and leaves us with 153601 free extents. We can now move this segment number from the last physical extent to the first extent. The command will thus be:

# pvmove --alloc anywhere /dev/sdd1:307201-399668 /dev/sdd1:0-92467
/dev/sdd1: Moved: 0.1 %
/dev/sdd1: Moved: 0.2 %
...
/dev/sdd1: Moved: 99.9 %
/dev/sdd1: Moved: 100.0 %
Note:
This command moves 399668 - 307201 + 1 = 92468 PEs from the last segment to the first segment. This is possible as the first segment encloses 153600 free PEs, which can contain the 92467 - 0 + 1 = 92468 moved PEs.
The --alloc anywhere option is used as we move PEs inside the same partition. In case of different partitions, the command would look something like this:
# pvmove /dev/sdb1:1000-1999 /dev/sdc1:0-999
This command may take a long time (one to two hours) in case of large volumes. It might be a good idea to run this command in a tmux or GNU Screen session. Any unwanted stop of the process could be fatal.
Once the operation is complete, run fsck to make sure your file system is valid.
Resize physical volume
Once all your free physical segments are on the last physical extents, run vgdisplay with root privileges and see your free PE.

Then you can now run again the command:

# pvresize --setphysicalvolumesize size PhysicalVolume
See the result:

# pvs
  PV         VG   Fmt  Attr PSize    PFree 
  /dev/sdd1  vg1  lvm2 a--     1t     500g
Resize partition
Last, you need to shrink the partition with your favorite partitioning tool.

Volume groups
Creating a volume group
To create a VG MyVolGroup with an associated PV /dev/sdb1, run:

# vgcreate MyVolGroup /dev/sdb1
You can check the VG MyVolGroup is created using the following command:

# vgs
You can bind multiple PVs when creating a VG like this:

# vgcreate MyVolGroup /dev/sdb1 /dev/sdb2
Activating a volume group
Note: You can restrict the volumes that are activated automatically by setting the auto_activation_volume_list in /etc/lvm/lvm.conf. If in doubt, leave this option commented out.
# vgchange -a y MyVolGroup
By default, this will reactivate the volume group when applicable. For example, if you had a drive failure in a mirror and you swapped the drive; and ran (1) pvcreate, (2) vgextend and (3) vgreduce --removemissing --force.

Repairing a volume group
To start the rebuilding process of the degraded mirror array in this example, you would run:

# lvconvert --repair /dev/MyVolGroup/mirror
You can monitor the rebuilding process (Cpy%Sync Column output) with:

# lvs -a -o +devices
Deactivating a volume group
Just invoke

# vgchange -a n MyVolGroup
This will deactivate the volume group and allow you to unmount the container it is stored in.

Renaming a volume group
Use the vgrename(8) command to rename an existing volume group.

Either of the following commands renames the existing volume group MyVolGroup to my_volume_group

# vgrename /dev/MyVolGroup /dev/my_volume_group
# vgrename MyVolGroup my_volume_group
Make sure to update all configuration files (e.g. /etc/fstab or /etc/crypttab) that reference the renamed volume group.

Add physical volume to a volume group
You first create a new physical volume on the block device you wish to use, then extend your volume group

# pvcreate /dev/sdb1
# vgextend MyVolGroup /dev/sdb1
This of course will increase the total number of physical extents on your volume group, which can be allocated by logical volumes as you see fit.

Note: It is considered good form to have a partition table on your storage medium below LVM. Use the appropriate partition type: 8e for MBR, and E6D6D379-F507-44C2-A23C-238F2A3DF928 for GPT partitions (8e00 type code in gdisk, lvm type alias in fdisk).
Remove partition from a volume group
If you created a logical volume on the partition, remove it first.

All of the data on that partition needs to be moved to another partition. Fortunately, LVM makes this easy:

# pvmove /dev/sdb1
If you want to have the data on a specific physical volume, specify that as the second argument to pvmove:

# pvmove /dev/sdb1 /dev/sdf1
Then the physical volume needs to be removed from the volume group:

# vgreduce MyVolGroup /dev/sdb1
Or remove all empty physical volumes:

# vgreduce --all MyVolGroup
For example: if you have a bad disk in a group that cannot be found because it has been removed or failed:

# vgreduce --removemissing --force MyVolGroup
And lastly, if you want to use the partition for something else, and want to avoid LVM thinking that the partition is a physical volume:

# pvremove /dev/sdb1
Logical volumes
Note: lvresize(8) provides more or less the same options as the specialized lvextend(8) and lvreduce(8) commands, while allowing to do both types of operation. Notwithstanding this, all those utilities offer a -r/--resizefs option which allows to resize the file system together with the LV using fsadm(8) (ext2, ext3, ext4, ReiserFS and XFS supported). Therefore it may be easier to simply use lvresize for both operations and use --resizefs to simplify things a bit, except if you have specific needs or want full control over the process.
Warning: While enlarging a file system can often be done on-line (i.e. while it is mounted), even for the root partition, shrinking will nearly always require to first unmount the file system so as to prevent data loss. Make sure your file system supports what you are trying to do.
Tip: If a logical volume will be formatted with ext4, leave at least 256 MiB free space in the volume group to allow using e2scrub(8). After creating the last volume with -l 100%FREE, this can be accomplished by reducing its size with lvreduce -L -256M volume_group/logical_volume.
Creating a logical volume
To create a LV homevol in a VG MyVolGroup with 300 GiB of capacity, run:

# lvcreate -L 300G MyVolGroup -n homevol
or, to create a LV homevol in a VG MyVolGroup with the rest of capacity, run:

# lvcreate -l 100%FREE MyVolGroup -n homevol
To create the LV while restricting it to specific PVs within the VG, append them to the command:

# lvcreate -L 300G MyVolGroup -n homevol /dev/sda1
The new LV will appear as /dev/MyVolGroup/homevol. Now you can format the LV with an appropriate file system.

You can check the LV is created using the following command:

# lvs
Renaming a logical volume
To rename an existing logical volume, use the lvrename(8) command.

Either of the following commands renames logical volume old_vol in volume group MyVolGroup to new_vol.

# lvrename /dev/MyVolGroup/old_vol /dev/MyVolGroup/new_vol
# lvrename MyVolGroup old_vol new_vol
Make sure to update all configuration files (e.g. /etc/fstab or /etc/crypttab) that reference the renamed logical volume.

Resizing the logical volume and file system in one go
Note: Only ext2, ext3, ext4, ReiserFS and XFS file systems are supported. For a different type of file system see #Resizing the logical volume and file system separately.
Extend the logical volume mediavol in MyVolGroup by 10 GiB and resize its file system all at once:

# lvresize -L +10G --resizefs MyVolGroup/mediavol
Set the size of logical volume mediavol in MyVolGroup to 15 GiB and resize its file system all at once:

# lvresize -L 15G --resizefs MyVolGroup/mediavol
If you want to fill all the free space on a volume group, use the following command:

# lvresize -l +100%FREE --resizefs MyVolGroup/mediavol
See lvresize(8) for more detailed options.

Resizing the logical volume and file system separately
For file systems not supported by fsadm(8) will need to use the appropriate utility to resize the file system before shrinking the logical volume or after expanding it.

To extend logical volume mediavol within volume group MyVolGroup by 2 GiB without touching its file system:

# lvresize -L +2G MyVolGroup/mediavol
Now expand the file system (ext4 in this example) to the maximum size of the underlying logical volume:

# resize2fs /dev/MyVolGroup/mediavol
For Btrfs, btrfs-filesystem(8) expects the mountpoint instead of the device, the equivalent is:

# btrfs filesystem resize max /mnt/my-mountpoint
To reduce the size of logical volume mediavol in MyVolGroup by 500 MiB, first calculate the resulting file system size and shrink the file system (Ext4 in this example) to the new size:

# resize2fs /dev/MyVolGroup/mediavol NewSize
Unlike Ext4, Btrfs supports online shrinking (again, a mountpoint should be specified) e.g.:

# btrfs filesystem resize -500M /mnt/my-mountpoint
When the file system is shrunk, reduce the size of logical volume:

# lvresize -L -500M MyVolGroup/mediavol
To calculate the exact logical volume size for ext2, ext3, ext4 file systems, use a simple formula: LVM_EXTENTS = FS_BLOCKS × FS_BLOCKSIZE ÷ LVM_EXTENTSIZE.

# tune2fs -l /dev/MyVolGroup/mediavol | grep Block
Block count:              102400000
Block size:               4096
Blocks per group:         32768
# vgdisplay MyVolGroup | grep "PE Size"
PE Size               4.00 MiB
Note: The file system block size is in bytes. Make sure to use the same units for both block and extent size.
102400000 blocks × 4096 bytes/block ÷ 4 MiB/extent = 100000 extents
Passing --resizefs will confirm that the correctness.

# lvreduce -l 100000 --resizefs /dev/MyVolGroup/mediavol
...
The filesystem is already 102400000 (4k) blocks long.  Nothing to do!
...
Logical volume sysvg/root successfully resized.
See lvresize(8) for more detailed options.

Removing a logical volume
Warning: Before you remove a logical volume, make sure to move all data that you want to keep somewhere else; otherwise, it will be lost!
First, find out the name of the logical volume you want to remove. You can get a list of all logical volumes with:

# lvs
Next, look up the mountpoint of the chosen logical volume:

$ lsblk
Then unmount the filesystem on the logical volume:

# umount /mountpoint
Finally, remove the logical volume:

# lvremove volume_group/logical_volume
For example:

# lvremove MyVolGroup/homevol
Confirm by typing in y.

Make sure to update all configuration files (e.g. /etc/fstab or /etc/crypttab) that reference the removed logical volume.

You can verify the removal of the logical volume by typing lvs as root again (see first step of this section).

Snapshots
LVM supports CoW (Copy-on-Write) snapshots. A CoW snapshot initially points to the original data. When data blocks are overwritten, the original copy is left intact and the new blocks are written elsewhere on-disk. This has several desirable properties:

Creating snapshots is fast, because it does not copy data (just the much shorter list of pointers to the on-disk locations).
Snapshots require just enough free space to hold the new data blocks (plus a negligible amount for the pointers to the new blocks). For example, a snapshot of 35 GiB of data, where you write only 2 GiB (on both the original and snapshot), only requires 2 GiB of free space.
LVM snapshots are at the block level. They make a new block device, with no apparent relationship to the original except when dealing with the LVM tools. Therefore, deleting files in the original copy does not free space in the snapshots. If you need filesystem-level snapshots, you rather need btrfs, ZFS or bcachefs.

Warning:
A CoW snapshot is not a backup, because it does not make a second copy of the original data. For example, a damaged disk sector that affects original data also affects the snapshots. That said, a snapshot can be helpful while using other tools to make backups, as outlined below.
Btrfs expects different filesystems to have different UUIDs. If you snapshot a LVM volume that contains a btrfs filesystem, make sure to change the UUID of the original or the copy, before both are mounted (or made visible to the kernel, for example if an unrelated daemon triggers a btrfs device scan). For details see btrfs wiki Gotcha's.
Configuration
You create snapshot logical volumes just like normal ones.

# lvcreate --size 100M --snapshot --name snap01vol /dev/MyVolGroup/lvol
With that volume, you may modify less than 100 MiB of data, before the snapshot volume fills up.

Reverting the modified lvol logical volume to the state when the snap01vol snapshot was taken can be done with

# lvconvert --merge /dev/MyVolGroup/snap01vol
In case the origin logical volume is active, merging will occur on the next reboot (merging can be done even from a LiveCD).

Note: The snapshot will no longer exist after merging.
Also multiple snapshots can be taken and each one can be merged with the origin logical volume at will.

Backups
A snapshot provides a frozen copy of a file system to make backups. For example, a backup taking two hours provides a more consistent image of the file system than directly backing up the partition.

The snapshot can be mounted and backed up with dd or tar. The size of the backup file done with dd will be the size of the files residing on the snapshot volume. To restore just create a snapshot, mount it, and write or extract the backup to it. And then merge it with the origin.

See Create root filesystem snapshots with LVM for automating the creation of clean root file system snapshots during system startup for backup and rollback.

This article or section needs expansion.

Reason: Show scripts to automate snapshots of root before updates, to rollback... updating menu.lst to boot snapshots (maybe in a separate article?) (Discuss in Talk:LVM)
Encryption
See dm-crypt/Encrypting an entire system#LUKS on LVM and dm-crypt/Encrypting an entire system#LVM on LUKS for the possible schemes of combining LUKS with LVM.

Cache
This article or section needs expansion.

Reason: LVM also supports --type writecache which uses dm-writecache. (Discuss in Talk:LVM)
From lvmcache(7):

The cache logical volume type uses a small and fast LV to improve the performance of a large and slow LV. It does this by storing the frequently used blocks on the faster LV. LVM refers to the small fast LV as a cache pool LV. The large slow LV is called the origin LV. Due to requirements from dm-cache (the kernel driver), LVM further splits the cache pool LV into two devices - the cache data LV and cache metadata LV. The cache data LV is where copies of data blocks are kept from the origin LV to increase speed. The cache metadata LV holds the accounting information that specifies where data blocks are stored (e.g. on the origin LV or on the cache data LV). Users should be familiar with these LVs if they wish to create the best and most robust cached logical volumes. All of these associated LVs must be in the same VG.
Create cache
Convert your fast disk (/dev/fastdisk) to PV and add to your existing VG (MyVolGroup):

# vgextend MyVolGroup /dev/fastdisk
Create a cache pool with automatic meta data on /dev/fastdisk and convert the existing LV MyVolGroup/rootvol to a cached volume, all in one step:

# lvcreate --type cache --cachemode writethrough -l 100%FREE -n root_cachepool MyVolGroup/rootvol /dev/fastdisk
Tip: Instead of using -l 100%FREE to allocate 100% of available space from PV /dev/fastdisk, you can use -L 20G instead to allocate only 20 GiB for cachepool.
Cachemode has two possible options:

writethrough ensures that any data written will be stored both in the cache pool LV and on the origin LV. The loss of a device associated with the cache pool LV in this case would not mean the loss of any data;
writeback ensures better performance, but at the cost of a higher risk of data loss in case the drive used for cache fails.
If a specific --cachemode is not indicated, the system will assume writethrough as default.

Tip: Cache hit and miss counts can be viewed with lvdisplay or alternatively with lvm-cache-stats from libblockdev-lvm which also shows them in percentages.
Remove cache
If you ever need to undo the one step creation operation above:

# lvconvert --uncache MyVolGroup/rootvol
This commits any pending writes still in the cache back to the origin LV, then deletes the cache. Other options are available and described in lvmcache(7).

RAID
LVM may be used to create a software RAID. It is a good choice if the user does not have hardware RAID and was planning on using LVM anyway. From lvmraid(7):

lvm(8) RAID is a way to create a Logical Volume (LV) that uses multiple physical devices to improve performance or tolerate device failures. In LVM, the physical devices are Physical Volumes (PVs) in a single Volume Group (VG).
LVM RAID supports RAID 0, RAID 1, RAID 4, RAID 5, RAID 6 and RAID 10. See Wikipedia:Standard RAID levels for details on each level.

Tip: mdadm may also be used to create a software RAID. It is arguably simpler, more popular, and easier to setup.
Setup RAID
Create physical volumes:

# pvcreate /dev/sda2 /dev/sdb2
Create volume group on the physical volumes:

# vgcreate MyVolGroup /dev/sda2 /dev/sdb2
New volumes
Create logical volumes using lvcreate --type raidlevel, see lvmraid(7) and lvcreate(8) for more options.

# lvcreate --type RaidLevel [OPTIONS] -n Name -L Size VG [PVs]
RAID 0
For example:

# lvcreate -n myraid1vol -i 2 -I 64 -L 70G VolGroup00 /dev/nvme1n1p1 /dev/nvme0n1p1
will create a 70 GiB striped (raid0) logical volume named "myraid1vol" in VolGroup00. Stripes will be spread over /dev/nvme1n1p1 and /dev/nvme0n1p1. Stripesize is set to be 64K.

RAID 1
For example:

# lvcreate --type raid1 --mirrors 1 -L 20G -n myraid1vol MyVolGroup /dev/sda2 /dev/sdb2
will create a 20 GiB mirrored logical volume named "myraid1vol" in VolGroup00 on /dev/sda2 and /dev/sdb2.

RAID 10
For example:

# lvcreate -n myraid1vol -L 100G --type raid10 -m 1 -i 2 MyVolGroup /dev/sdd1 /dev/sdc1 /dev/sdb1 /dev/sda5
will create a 100 GiB RAID10 logical volume named "myraid1vol" in VolGroup00 on /dev/sdd1, /dev/sdc1, /dev/sdb1, and /dev/sda5.

Existing volumes
You can convert easily a non-RAID (e.g. linear) volume to pretty much any other raid configuration provided that you have enough physical devices to meet the RAID requirements. Some of them will require you to go through intermediate steps which lvconvert will inform you about and prompt you to agree. raid10 below can be replaced with raid0, raid1, raid5 etc.

# lvconvert --type raid10 /dev/vg01/lv01
Use specific PVs:

# lvconvert --type raid10 /dev/vg01/lv01 /dev/sda1 /dev/sdb2 /dev/nvme0n1p1 ...
You can keep track of the progress of conversion with:

# watch lvs -o name,vg_name,copy_percent
Thin provisioning
Note: When mounting a thin LV file system, always remember to use the discard option or to use fstrim regularly, to allow the thin LV to shrink as files are deleted.
From lvmthin(7):

Blocks in a standard lvm(8) Logical Volume (LV) are allocated when the LV is created, but blocks in a thin provisioned LV are allocated as they are written. Because of this, a thin provisioned LV is given a virtual size, and can then be much larger than physically available storage. The amount of physical storage provided for thin provisioned LVs can be increased later as the need arises.
Example: implementing virtual private servers
Here is the classic use case. Suppose you want to start your own VPS service, initially hosting about 100 VPSes on a single PC with a 930 GiB hard drive. Hardly any of the VPSes will actually use all of the storage they are allotted, so rather than allocate 9 GiB to each VPS, you could allow each VPS a maximum of 30 GiB and use thin provisioning to only allocate as much hard drive space to each VPS as they are actually using. Suppose the 930 GiB hard drive is /dev/sdb. Here is the setup.

Prepare the volume group, MyVolGroup.

# vgcreate MyVolGroup /dev/sdb
Create the thin pool LV, MyThinPool. This LV provides the blocks for storage.

# lvcreate --type thin-pool -n MyThinPool -l 95%FREE MyVolGroup
The thin pool is composed of two sub-volumes, the data LV and the metadata LV. This command creates both automatically. But the thin pool stops working if either fills completely, and LVM currently does not support the shrinking of either of these volumes. This is why the above command allows for 5% of extra space, in case you ever need to expand the data or metadata sub-volumes of the thin pool.

For each VPS, create a thin LV. This is the block device exposed to the user for their root partition.

# lvcreate -n SomeClientsRoot -V 30G --thinpool MyThinPool MyVolGroup
The block device /dev/MyVolGroup/SomeClientsRoot may then be used by a VirtualBox instance as the root partition.

Use thin snapshots to save more space
Thin snapshots are much more powerful than regular snapshots, because they are themselves thin LVs. See Red Hat's guide [4] for a complete list of advantages thin snapshots have.

Instead of installing Linux from scratch every time a VPS is created, it is more space-efficient to start with just one thin LV containing a basic installation of Linux:

# lvcreate -n GenericRoot -V 30G --thinpool MyThinPool MyVolGroup
*** install Linux at /dev/MyVolGroup/GenericRoot ***
Then create snapshots of it for each VPS:

# lvcreate -s MyVolGroup/GenericRoot -n SomeClientsRoot
This way, in the thin pool there is only one copy the data common to all VPSes, at least initially. As an added bonus, the creation of a new VPS is instantaneous.

Since these are thin snapshots, a write operation to GenericRoot only causes one COW operation in total, instead of one COW operation per snapshot. This allows you to update GenericRoot more efficiently than if each VPS were a regular snapshot.

Example: zero-downtime storage upgrade
There are applications of thin provisioning outside of VPS hosting. Here is how you may use it to grow the effective capacity of an already-mounted file system without having to unmount it. Suppose, again, that the server has a single 930 GiB hard drive. The setup is the same as for VPS hosting, only there is only one thin LV and the LV's size is far larger than the thin pool's size.

# lvcreate -n MyThinLV -V 16T --thinpool MyThinPool MyVolGroup
This extra virtual space can be filled in with actual storage at a later time by extending the thin pool.

Suppose some time later, a storage upgrade is needed, and a new hard drive, /dev/sdc, is plugged into the server. To upgrade the thin pool's capacity, add the new hard drive to the VG:

# vgextend MyVolGroup /dev/sdc
Now, extend the thin pool:

# lvextend -l +95%FREE MyVolGroup/MyThinPool
Since this thin LV's size is 16 TiB, you could add another 15.09 TiB of hard drive space before finally having to unmount and resize the file system.

Note: You will probably want to use reserved blocks or a disk quota to prevent applications from attempting to use more physical storage than there actually is.
Customizing
Some customisation is available by editing /etc/lvm/lvm.conf. You may find it useful to customize the output of lvs and pvs which by default does not include the % sync (useful to see progress of conversion between e.g. linear and raid type) and type of logical volume:

/etc/lvm/lvm.conf
report {
 	lvs_cols = "lv_name,lv_attr,lv_active,vg_name,lv_size,lv_layout,lv_allocation_policy,copy_percent,chunk_size"
	pvs_cols = "pv_name,vg_name,pv_size,pv_free,pv_used,dev_size"
}
Troubleshooting
LVM commands do not work
Load proper module:
# modprobe dm_mod
The dm_mod module should be automatically loaded. In case it does not, explicitly load the module at boot.

Try preceding commands with lvm like this:
# lvm pvdisplay
Logical volumes do not show up
If you are trying to mount existing logical volumes, but they do not show up in lvscan, you can use the following commands to activate them:

# vgscan
# vgchange -ay
LVM on removable media
Symptoms:

# vgscan
  Reading all physical volumes.  This may take a while...
  /dev/backupdrive1/backup: read failed after 0 of 4096 at 319836585984: Input/output error
  /dev/backupdrive1/backup: read failed after 0 of 4096 at 319836643328: Input/output error
  /dev/backupdrive1/backup: read failed after 0 of 4096 at 0: Input/output error
  /dev/backupdrive1/backup: read failed after 0 of 4096 at 4096: Input/output error
  Found volume group "backupdrive1" using metadata type lvm2
  Found volume group "networkdrive" using metadata type lvm2
Cause: removing an external LVM drive without deactivating the volume group(s) first. Before you disconnect, make sure to:

# vgchange -an volume_group_name
Fix: assuming you already tried to activate the volume group with vgchange -ay vg, and are receiving the Input/output errors:

# vgchange -an volume_group_name
Unplug the external drive and wait a few minutes:

# vgscan
# vgchange -ay volume_group_name
Suspend/resume with LVM and removable media
The factual accuracy of this article or section is disputed.

Reason: Provided solution will not work in more complex setups like LUKS on LVM. (Discuss in Talk:LVM#LVM on removable media)
In order for LVM to work properly with removable media – like an external USB drive – the volume group of the external drive needs to be deactivated before suspend. If this is not done, you may get buffer I/O errors on the dm device (after resume). For this reason, it is not recommended to mix external and internal drives in the same volume group.

To automatically deactivate the volume groups with external USB drives, tag each volume group with the sleep_umount tag in this way:

# vgchange --addtag sleep_umount vg_external
Once the tag is set, use the following unit file for systemd to properly deactivate the volumes before suspend. On resume, they will be automatically activated by LVM.

/etc/systemd/system/ext_usb_vg_deactivate.service
[Unit]
Description=Deactivate external USB volume groups on suspend
Before=sleep.target

[Service]
Type=oneshot
ExecStart=-/etc/systemd/system/deactivate_sleep_vgs.sh

[Install]
WantedBy=sleep.target
and this script:

/etc/systemd/system/deactivate_sleep_vgs.sh
#!/bin/sh

TAG=@sleep_umount
vgs=$(vgs --noheadings -o vg_name $TAG)

echo "Deactivating volume groups with $TAG tag: $vgs"

# Unmount logical volumes belonging to all the volume groups with tag $TAG
for vg in $vgs; do
    for lv_dev_path in $(lvs --noheadings  -o lv_path -S lv_active=active,vg_name=$vg); do
        echo "Unmounting logical volume $lv_dev_path"
        umount $lv_dev_path
    done
done

# Deactivate volume groups tagged with sleep_umount
for vg in $vgs; do
    echo "Deactivating volume group $vg"
    vgchange -an $vg
done
Finally, enable the unit.

Resizing a contiguous logical volume fails
If trying to extend a logical volume errors with:

" Insufficient suitable contiguous allocatable extents for logical volume "
The reason is that the logical volume was created with an explicit contiguous allocation policy (options -C y or --alloc contiguous) and no further adjacent contiguous extents are available.[5]

To fix this, prior to extending the logical volume, change its allocation policy with lvchange --alloc inherit logical_volume. If you need to keep the contiguous allocation policy, an alternative approach is to move the volume to a disk area with sufficient free extents. See [6].

Command "grub-mkconfig" reports "unknown filesystem" errors
Make sure to remove snapshot volumes before generating grub.cfg.

Thinly-provisioned root volume device times out
With a large number of snapshots, thin_check runs for a long enough time so that waiting for the root device times out. To compensate, add the rootdelay=60 kernel boot parameter to your boot loader configuration. Or, make thin_check skip checking block mappings (see [7]) and regenerate the initramfs:

/etc/lvm/lvm.conf
thin_check_options = [ "-q", "--clear-needs-check-flag", "--skip-mappings" ]
Delay on shutdown
If you use RAID, snapshots or thin provisioning and experience a delay on shutdown, make sure lvm2-monitor.service is started. See FS#50420.

Hibernating into a thinly-provisioned swap volume
See Power management/Suspend and hibernate#Hibernation into a thinly-provisioned LVM volume.