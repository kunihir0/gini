ibvirt is a collection of software that provides a convenient way to manage virtual machines and other virtualization functionality, such as storage and network interface management. These software pieces include a long term stable C API, a daemon (libvirtd), and a command line utility (virsh). A primary goal of libvirt is to provide a single way to manage multiple different virtualization providers/hypervisors, such as the KVM/QEMU, Xen, LXC, OpenVZ or VirtualBox hypervisors (among others).

Some of the major libvirt features are:

Virtual machine management: Various domain lifecycle operations such as start, stop, pause, save, restore, and migrate. Hotplug operations for many device types including disk and network interfaces, memory, and CPUs.
Remote machine support: All libvirt functionality is accessible on any machine running the libvirt daemon, including remote machines. A variety of network transports are supported for connecting remotely, with the simplest being SSH, which requires no extra explicit configuration.
Storage management: Any host running the libvirt daemon can be used to manage various types of storage: create file images of various formats (qcow2, vmdk, raw, ...), mount NFS shares, enumerate existing LVM volume groups, create new LVM volume groups and logical volumes, partition raw disk devices, mount iSCSI shares, and much more.
Network interface management: Any host running the libvirt daemon can be used to manage physical and logical network interfaces. Enumerate existing interfaces, as well as configure (and create) interfaces, bridges, vlans, and bond devices.
Virtual NAT and Route based networking: Any host running the libvirt daemon can manage and create virtual networks. Libvirt virtual networks use firewall rules to act as a router, providing VMs transparent access to the host machines network.
Installation
Because of its daemon/client architecture, libvirt needs only be installed on the machine which will host the virtualized system. Note that the server and client can be the same physical machine.

Server
Install the libvirt package, as well as at least one hypervisor:

The libvirt KVM/QEMU driver is the primary libvirt driver and if KVM is enabled, fully virtualized, hardware accelerated guests will be available. See the QEMU article for more information.
Other supported hypervisors include LXC, VirtualBox and Xen. See the respective articles for installation instructions. With respect to libvirtd installation note:
The libvirt LXC driver has no dependency on the LXC userspace tools provided by lxc, therefore there is no need to install the package if planning on using the driver. libvirtd needs to be running to use libvirt-lxc connection.
Xen support is available, but not by default (FS#27356). You need to use the ABS to modify libvirt's PKGBUILD and build it without the -Ddriver_libxl=disabled option.
For network connectivity, install:

dnsmasq for the default NAT/DHCP networking.
openbsd-netcat for remote management over SSH.
Other optional dependencies may provide desired or extended features, such as dmidecode for DMI system info support. Install the ones you may need as dependencies after reading pacman's output for libvirt.

Note: If you are using firewalld, as of libvirt 5.1.0 and firewalld 0.7.0 you no longer need to change the firewall backend to iptables. libvirt now installs a zone called 'libvirt' in firewalld and manages its required network rules there. See Firewall and network filtering in libvirt.
Client
The client is the user interface that will be used to manage and access the virtual machines.

virsh — Command line program for managing and configuring domains.
https://libvirt.org/ || libvirt
Boxes — Simple GNOME application to access virtual systems. Part of gnome-extra.
https://apps.gnome.org/Boxes/ || gnome-boxes
Libvirt Sandbox — Application sandbox toolkit.
https://sandbox.libvirt.org/ || libvirt-sandboxAUR
Virt Viewer — Simple remote display client.
https://gitlab.com/virt-viewer/virt-viewer || virt-viewer
Virt-manager — Graphically manage KVM, Xen, or LXC via libvirt.
https://virt-manager.org/ || virt-manager
Cockpit — Web-based system administration tool with plugin to manage virtual machines.
https://cockpit-project.org/ || cockpit-machines
A list of libvirt-compatible software can be found here.

Configuration
Libvirt can manage QEMU virtual machines in two modes, system and session[1][2]:

system URIs connect to the libvirtd daemon running as root which is launched at system startup. Virtual machines created and run using 'system' are usually launched as root, unless configured otherwise (for example in /etc/libvirt/qemu.conf)
session URIs launch a libvirtd instance as your local user, and all VMs are run with local user permissions.
Regarding their pros and cons:

Virtual machine autostarting on host boot only works for 'system', and the root libvirtd instance has necessary permissions to use proper networking via bridges or virtual networks. qemu:///system is generally what tools like virt-manager default to.
qemu:///session has a serious drawback: since the libvirtd instance does not have sufficient privileges, the only out of the box network option is qemu's usermode networking, which has non-obvious limitations, so its usage is discouraged (more info on qemu networking options)
The benefit of qemu:///session is that permission issues vanish: disk images can easily be stored in $HOME, serial PTYs are owned by the user, etc.
For system-level administration (i.e. global settings and image-volume location), libvirt minimally requires setting up authorization, and starting the daemon.

For user-session administration, daemon setup and configuration is not required; however, authorization is limited to local abilities; the front-end will launch a local instance of the libvirtd daemon.

Set up authentication
From libvirt: Connection authentication:

The libvirt daemon allows the administrator to choose the authentication mechanisms used for client connections on each network socket independently. This is primarily controlled via the libvirt daemon master config file in /etc/libvirt/libvirtd.conf. Each of the libvirt sockets can have its authentication mechanism configured independently. There is currently a choice of none, polkit and sasl.
Using libvirt group
The easiest way to ensure your user has access to libvirt daemon is to add member to libvirt user group.

Members of the libvirt group have passwordless access to the RW daemon socket by default.

Using polkit
Because libvirt pulls polkit as a dependency during installation, polkit is used as the default value for the unix_sock_auth parameter (source). File-based permissions remain nevertheless available.

Note: A system reboot may be required before authenticating with polkit works correctly.
The libvirt daemon provides two polkit actions in /usr/share/polkit-1/actions/org.libvirt.unix.policy:

org.libvirt.unix.manage for full management access (RW daemon socket), and
org.libvirt.unix.monitor for monitoring only access (read-only socket).
The default policy for the RW daemon socket will require to authenticate as an admin. This is akin to sudo auth, but does not require that the client application ultimately run as root. Default policy will still allow any application to connect to the RO socket.

Arch defaults to consider anybody in the wheel group as an administrator: this is defined in /usr/share/polkit-1/rules.d/50-default.rules (see Polkit#Administrator identities). Therefore there is no need to create a new group and rule file if your user is a member of the wheel group: upon connection to the RW socket (e.g. via virt-manager) you will be prompted for your user's password.

Note: Prompting for a password relies on the presence of an authentication agent on the system. Console users may face an issue with the default pkttyagent agent which may or may not work properly.
Tip: If you want to configure passwordless authentication, see Polkit#Bypass password prompt.
You may change the group authorized to access the RW daemon socket. As an example, to authorize the mykvm group, create the following file:

/etc/polkit-1/rules.d/50-libvirt.rules
/* Allow users in mykvm group to manage the libvirt
daemon without authentication */
polkit.addRule(function(action, subject) {
    if (action.id == "org.libvirt.unix.manage" &&
        subject.isInGroup("mykvm")) {
            return polkit.Result.YES;
    }
});
Then add yourself to the mykvm group and relogin. Replace mykvm with any group of your preference just make sure it exists and that your user is a member of it (see Users and groups for more information).

Do not forget to relogin for group changes to take effect.

Authenticate with file-based permissions
To define file-based permissions for users in the libvirt group to manage virtual machines, uncomment and define:

/etc/libvirt/libvirtd.conf
#unix_sock_group = "libvirt"
#unix_sock_ro_perms = "0777"  # set to 0770 to deny non-group libvirt users
#unix_sock_rw_perms = "0770"
#auth_unix_ro = "none"
#auth_unix_rw = "none"
While some guides mention changed permissions of certain libvirt directories to ease management, keep in mind permissions are lost on package update. To edit these system directories, root user is expected.

Daemon
Note: Libvirt is moving from a single monolithic daemon to separate modular daemons, with the intention to remove the monolithic daemon in the future. See Libvirt daemons for more infomation.
Start both libvirtd.service and virtlogd.service. Optionally enable libvirtd.service (which will also enable virtlogd.socket and virtlockd.socket units, so there is NO need to also enable virtlogd.service).

Another possibility is to only start/enable libvirtd.socket and virtlogd.socket for on-demand socket activation.

Unencrypt TCP/IP sockets
Warning: This method is used to help remote domain, connection speed for trusted networks. This is the least secure connection method. This should only be used for testing or use over a secure, private, and trusted network. SASL is not enabled here, so all TCP traffic is cleartext. For real world use always enable SASL.
Edit /etc/libvirt/libvirtd.conf:

/etc/libvirt/libvirtd.conf
listen_tls = 0
listen_tcp = 1
auth_tcp="none"
It is also necessary to start the server in listening mode by editing /etc/conf.d/libvirtd:

/etc/conf.d/libvirtd
LIBVIRTD_ARGS="--listen"
Access virtual machines using their hostnames
For host access to guests on non-isolated, bridged networks, enable the libvirt and/or libvirt_guest NSS modules provided by libvirt. For the comparison of the two modules and technical details, see libvirt documentation.

Add desired modules in nsswitch.conf(5):

/etc/nsswitch.conf
hosts: files libvirt libvirt_guest dns myhostname
Note: While commands such as ping and ssh should work with virtual machine hostnames, commands such as host and nslookup may fail or produce unexpected results because they rely on DNS. Use getent hosts <vm-hostname> instead.
Test
To test if libvirt is working properly on a system level:

$ virsh -c qemu:///system
To test if libvirt is working properly for a user-session:

$ virsh -c qemu:///session
Management
Libvirt management is done mostly with three tools: virt-manager (GUI), virsh, and guestfish (which is part of libguestfs).

virsh
The virsh program is for managing guest domains (virtual machines) and works well for scripting, virtualization administration. Though most virsh commands require root privileges to run due to the communication channels used to talk to the hypervisor, typical management, creation, and running of domains (like that done with VirtualBox) can be done as a regular user.

The virsh program includes an interactive terminal that can be entered if no commands are passed (options are allowed though): virsh. The interactive terminal has support for tab completion.

From the command line:

$ virsh [option] <command> [argument]...
From the interactive terminal:

virsh # <command> [argument]...
Help is available:

$ virsh help [option*] or [group-keyword*]
Storage pools
A pool is a location where storage volumes can be kept. What libvirt defines as volumes others may define as "virtual disks" or "virtual machine images". Pool locations may be a directory, a network filesystem, or partition (this includes a LVM). Pools can be toggled active or inactive and allocated for space.

On the system-level, /var/lib/libvirt/images/ will be activated by default; on a user-session, virt-manager creates $XDG_DATA_HOME/images.

Print active and inactive storage pools:

$ virsh pool-list --all
Create a new pool using virsh
If one wanted to add a storage pool, here are examples of the command form, adding a directory, and adding a LVM volume:

$ virsh pool-define-as name type [source-host] [source-path] [source-dev] [source-name] [<target>] [--source-format format]
$ virsh pool-define-as poolname dir - - - - /home/username/.local/libvirt/images
$ virsh pool-define-as poolname fs - -  /dev/vg0/images - mntpoint
The above command defines the information for the pool, to build it:

$ virsh pool-build     poolname
$ virsh pool-start     poolname
$ virsh pool-autostart poolname
To remove it:

$ virsh pool-undefine  poolname
Tip: For LVM storage pools:
It is a good practice to dedicate a volume group to the storage pool only.
Choose a LVM volume group that differs from the pool name, otherwise when the storage pool is deleted the LVM group will be too.
Create a new pool using virt-manager
First, connect to a hypervisor (e.g. QEMU/KVM system, or user-session). Then, right-click on a connection and select Details; select the Storage tab, push the + button on the lower-left, and follow the wizard.

Storage volumes
Once the pool has been created, volumes can be created inside the pool. If building a new domain (virtual machine), this step can be skipped as a volume can be created in the domain creation process.

Create a new volume with virsh
Create volume, list volumes, resize, and delete:

$ virsh vol-create-as      poolname volumename 10GiB --format aw|bochs|raw|qcow|qcow2|vmdk
$ virsh vol-upload  --pool poolname volumename volumepath
$ virsh vol-list           poolname
$ virsh vol-resize  --pool poolname volumename 12GiB
$ virsh vol-delete  --pool poolname volumename
$ virsh vol-dumpxml --pool poolname volumename  # for details.
Domains
Virtual machines are called domains. If working from the command line, use virsh to list, create, pause, shutdown domains, etc. virt-viewer can be used to view domains started with virsh. Creation of domains is typically done either graphically with virt-manager or with virt-install (a command line program installed as part of the virt-install package).

Creating a new domain typically involves using some installation media, such as an .iso from the storage pool or an optical drive.

Print active and inactive domains:

# virsh list --all
Note: SELinux has a built-in exemption for libvirt that allows volumes in /var/lib/libvirt/images/ to be accessed. If using SELinux and there are issues with the volumes, ensure that volumes are in that directory, or ensure that other storage pools are correctly labeled.
Create a new domain using virt-install
The factual accuracy of this article or section is disputed.

Reason: /usr/share/libosinfo is not provided by any official packages, including libosinfo. (Discuss in Talk:Libvirt#Where_is_'/usr/share/libosinfo/db/oses/os.xml'？)
For an extremely detailed domain (virtual machine) setup, it is easier to #Create a new domain using virt-manager. However, basics can easily be done with virt-install and still run quite well. Minimum specifications are --name, --memory, guest storage (--disk, --filesystem, or --nodisks), and an install method (generally an .iso or CD). See virt-install(1) for more details and information about unlisted options.

Arch Linux install (two GiB, qcow2 format volume create; user-networking):

$ virt-install  \
  --name arch-linux_testing \
  --memory 1024             \
  --vcpus=2,maxvcpus=4      \
  --cpu host                \
  --cdrom $HOME/Downloads/arch-linux_install.iso \
  --disk size=2,format=qcow2  \
  --network user            \
  --virt-type kvm
Fedora testing (Xen hypervisor, non-default pool, do not originally view):

$ virt-install  \
  --connect xen:///     \
  --name fedora-testing \
  --memory 2048         \
  --vcpus=2             \
  --cpu=host            \
  --cdrom /tmp/fedora20_x84-64.iso      \
  --os-type=linux --os-variant=fedora20 \
  --disk pool=testing,size=4            \
  --network bridge=br0                  \
  --graphics=vnc                        \
  --noautoconsole
$ virt-viewer --connect xen:/// fedora-testing
Windows:

$ virt-install \
  --name=windows7           \
  --memory 2048             \
  --cdrom /dev/sr0          \
  --os-variant=win7         \
  --disk /mnt/storage/domains/windows7.qcow2,size=20GiB \
  --network network=vm-net  \
  --graphics spice
Tip: Run osinfo-query --fields=name,short-id,version os to get argument for --os-variant; this will help define some specifications for the domain. However, --memory and --disk will need to be entered; one can look within the appropriate /usr/share/libosinfo/db/oses/os.xml if needing these specifications. After installing, it will likely be preferable to install the Spice Guest Tools that include the VirtIO drivers. For a Windows VirtIO network driver there is also virtio-winAUR. These drivers are referenced by a <model type='virtio' /> in the guest's .xml configuration section for the device. A bit more information can also be found on the QEMU article.
Import existing volume:

$ virt-install  \
  --name demo  \
  --memory 512 \
  --disk /home/user/VMs/mydisk.img \
  --import
Create a new domain using virt-manager
First, connect to the hypervisor (e.g. QEMU/KVM system or user session), right click on a connection and select New, and follow the wizard.

On the fourth step, de-selecting Allocate entire disk now will make setup quicker and can save disk space in the interum; however, it may cause volume fragmentation over time.
On the fifth step, open Advanced options and make sure that Virt Type is set to kvm (this is usually the preferred method). If additional hardware setup is required, select the Customize configuration before install option.
Manage a domain
Start a domain:

$ virsh start domain
$ virt-viewer --connect qemu:///session domain
Gracefully attempt to shutdown a domain; force off a domain:

$ virsh shutdown domain
$ virsh destroy  domain
Autostart domain on libvirtd start:

$ virsh autostart domain
$ virsh autostart domain --disable
Shutdown domain on host shutdown:

Running domains can be automatically suspended/shutdown at host shutdown using the libvirt-guests.service systemd service. This same service will resume/startup the suspended/shutdown domain automatically at host startup. See libvirt-guests(8) for details and service options.
Edit a domain's XML configuration:

$ virsh edit domain
To know more about XML configurations read the XML format section of the libvirt wiki.

Note: Virtual Machines started directly by QEMU are not manageable by libvirt tools.
Networking
Virtual networks
Virtual networks are used to connect domains to either internal or external networks. The bridge device is used to define the virtual network. Additionally, the forwarding mode is used to define the internal or external networks a domain is able to reach.

Forwarding modes
Some common forwarding modes are listed below:

Mode	Description
Bridge	The virtual network is connected to the same network segment as the host.
NAT	The virtual network uses the host's networking stack, uses NAT, and inbound connections are restricted.
Routed	The virtual network uses the host's networking stack, and inbound connections are restricted.
Open	The virtual network uses the host's networking stack.
Isolated	No other networks are reachable from the virtual network.
Using iptables
If iptables is to be used and not nftables, it is necessary to specify this accordingly in the configuration file: /etc/libvirt/network.conf.

For example':

# default: #firewall_backend = "nftables"
firewall_backend = "iptables"
Retrieving a domain IP address
If using the default network and addresses are assigned using DHCP:

$ virsh net-dhcp-leases default
If the domain is using the qemu-guest-agent:

$ virsh domifaddr --source agent domain
Using nftables
When using network type NAT in combination with a simple nftables firewall, you may need to allow forwarding to/from the virtual network interface, and allow DNS/DHCP requests for DHCP clients from the virtual network interface to the host.

The relevant sections of nftables.conf are below:

/etc/nftables.conf
# ...
table inet filter {
  chain input {
    type filter hook input priority filter
    policy drop
    # ...
    iifname virbr0 udp dport {53, 67} accept comment "allow VM dhcp/dns requests to host"
    # ...
  }

  chain forward {
    type filter hook forward priority filter
    policy drop
    
    iifname virbr0 accept
    oifname virbr0 accept
  }
}
Adding an IPv6 address
When adding an IPv6 address through any of the configuration tools, you will likely receive the following error:

Check the host setup: enabling IPv6 forwarding with RA routes without accept_ra set to 2 is likely to cause routes loss. Interfaces to look at: eth0
Fix this by running the following command (replace eth0 with the name of your physical interface):

# sysctl net.ipv6.conf.eth0.accept_ra=2
Port forwarding to domains
Details on how to do this can be found in the libvirt NAT forwarding documentation.

Warning: If using the NAT network type, incoming connections will be prohibited. It is therefore recommended to use the route or open network types.
Snapshots
Snapshots take the disk, memory, and device state of a domain at a point-of-time, and save it for future use. They have many uses, from saving a "clean" copy of an OS image to saving a domain's state before a potentially destructive operation. Snapshots are identified with a unique name.

Snapshots are saved within the volume itself and the volume must be the format: qcow2 or raw. Snapshots use deltas in order not to take as much space as a full copy would.

Create a snapshot
This article or section is out of date.

Reason: Some of this data appears to be dated. (Discuss in Talk:Libvirt)
Once a snapshot is taken it is saved as a new block device and the original snapshot is taken offline. Snapshots can be chosen from and also merged into another (even without shutting down the domain).

Print a running domain's volumes (running domains can be printed with virsh list):

# virsh domblklist domain
 Target     Source
 ------------------------------------------------
 vda        /vms/domain.img
To see a volume's physical properties:

# qemu-img info /vms/domain.img
 image: /vms/domain.img
 file format: qcow2
 virtual size: 50G (53687091200 bytes)
 disk size: 2.1G
 cluster_size: 65536
Create a disk-only snapshot (the option --atomic will prevent the volume from being modified if snapshot creation fails):

# virsh snapshot-create-as domain snapshot1 --disk-only --atomic
List snapshots:

# virsh snapshot-list domain
 Name                 Creation Time             State
 ------------------------------------------------------------
 snapshot1           2012-10-21 17:12:57 -0700 disk-snapshot
One can then copy the original image with cp --sparse=true or rsync -S and then merge the original back into snapshot:

# virsh blockpull --domain domain --path /vms/domain.snapshot1
domain.snapshot1 becomes a new volume. After this is done the original volume (domain.img and snapshot metadata can be deleted. The virsh blockcommit would work opposite to blockpull but it seems to be currently under development (including snapshot-revert feature, scheduled to be released sometime next year.

Other management
Connect to non-default hypervisor:

$ virsh --connect xen:///
virsh # uri
xen:///
Connect to the QEMU hypervisor over SSH; and the same with logging:

$ virsh --connect qemu+ssh://username@host/system
$ LIBVIRT_DEBUG=1 virsh --connect qemu+ssh://username@host/system
Connect a graphic console over SSH:

$ virt-viewer  --connect qemu+ssh://username@host/system domain
$ virt-manager --connect qemu+ssh://username@host/system domain
Note: If you are having problems connecting to a remote RHEL server (or anything other than Arch, really), try the two workarounds mentioned in FS#30748 and FS#22068.
Connect to the VirtualBox hypervisor (VirtualBox support in libvirt is not stable yet and may cause libvirtd to crash):

$ virsh --connect vbox:///system
Network configurations:

$ virsh -c qemu:///system net-list --all
$ virsh -c qemu:///system net-dumpxml default
Hooks
Hooks are scripts that are triggered by different events happening while starting and running the libvirt daemon. They can be used to execute commands needed in preparation to launch a guest like setup networks or reserve memory.

The following hooks exists:

daemon - occasions to trigger: start, shutdown, reload
qemu - occasions to trigger: prepare, prepare, start, started, stopped, release, migrate, restore, reconnect, attach
lxc - occasions to trigger: prepare, start, started, stopped, release, reconnect
libxl - occasions to trigger: prepare, start, started, stopped, release migrate, reconnect
network - occasions to trigger: start, started, stopped, port-created, updated, port-deleted
See the libvirt Documentation for details about each hook and trigger.

Create a hook
Hooks are represented by scripts located at /etc/libvirt/hooks. If the folder does not exist, you have to create it. Each hook is represented by a script in this folder with the same name (e.g. /etc/libvirt/hooks/qemu) or a sub-folder (e.g. /etc/libvirt/hooks/qemu.d/). The later can contain different scripts, which are all run at the trigger points. The scripts are run like any other scripts, so they need to start with the declaration of the command interpreter to use (e.g. #!/bin/bash) and be executable by the libvirt user.

Every time a trigger point is met, the script is run. For example, the daemon script would run at least two times in a start/stop cycle of the system, at start and at shutdown. To run an command only at a given point, you have to implement conditions in the script. To do this, libvirt passes parameters which can be used to identify the current trigger condition.

According to the libvirt documentation these parameters are defined as follows:

Parameter 1: The name of the object involved in the operation
Parameter 2: The name of the operation being performed
Parameter 3: Used if a sub-operation is to be named
Parameter 4: An extra argument if needed
If one of the arguments is not applicable, a dash is passed.

Note: If the hooks are not working after creating your script, try restarting the libvirt daemon. With the new modular daemons, the daemon to restart depends on the hook (e.g. virtqemud for the qemu hook).
Example
To run an command every time you start an qemu guest, before any resources are allocated, you can use the qemu hook. At this point, libvirt runs the hooks like this: /etc/libvirt/hooks/qemu <guest_name> prepare begin - The script for this could like this:

/etc/libvirt/hooks/qemu
#!/bin/bash
guest_name="$1"
libvirt_task="$2"
if [ "$libvirt_task" = "prepare" ]; then
	<run some important code here>
fi
If the guest is stopped, the same script would be run, but this time the daemon would start the command like this: /etc/libvirt/hooks/qemu <guest_name> stopped end -

Sharing data between host and guest
Virtio-FS
Sharing files with Virtio-FS lists an overview of the supported options to enable filesharing with the guest.

Set up the memory backend
Memory backends must be allocated before using virtiofs. memfd and file-backed memory backends can be used in system sessions and unprivileged QEMU/KVM user sessions. Hugepages only supports system sessions.

memfd
To use memfd memory backend, you need to add the following domain XML elements:

# virsh edit name_of_virtual_machine
<domain>
  ...
  <memoryBacking>
    <source type='memfd'/>
    <access mode='shared'/>
  </memoryBacking>
  ...
</domain>

file-backed
Add the following domain XML elements:

# virsh edit name_of_virtual_machine
<domain>
  ...
  <memoryBacking>
    <access mode='shared'/>
  </memoryBacking>
  ...
</domain>

You can configure where the backing file is stored with the memory_backing_dir option in /etc/libvirt/qemu.conf or, if you are running a user session, in $XDG_CONFIG_HOME/libvirt/qemu.conf:

memory_backing_dir = "/dev/shm/"
hugepage
Note: hugepage is not supported in QEMU/KVM user sessions.
First you need to enable hugepages which are used by the virtual machine:

/etc/sysctl.d/40-hugepage.conf
vm.nr_hugepages = nr_hugepages
To determine the number of hugepages needed check the size of the hugepages:

$ grep Hugepagesize /proc/meminfo
The number of hugepages is memory size of virtual machine / Hugepagesize. Add to this value some additional pages. You have to reboot after this step, so that the hugepages are allocated.

Now you have to prepare the configuration of the virtual machine:

# virsh edit name_of_virtual_machine
<domain>
  ...
  <memoryBacking>
    <hugepages/>
  </memoryBacking>
  ...
  <cpu ...>
    <numa>
      <cell memory='memory size of virtual machine' unit='KiB' memAccess='shared'/>
    </numa>
  </cpu>
  ...
</domain>
It is necessary to add the NUMA definition so that the memory access can be declared as shared. id and cpus values for NUMA will be inserted by virsh.

Configure filesystem passthrough
Add the following domain XML elements:

# virsh edit name_of_virtual_machine
<domain>
...
  <devices>
    ...
    <filesystem type='mount' accessmode='passthrough'>
      <driver type='virtiofs'/>
      <source dir='path/to/folder/on/host'/>
      <target dir='mount_tag'/>
    </filesystem>
    ...
  </devices>
</domain>
Replace path/to/folder/on/host with the directory you want to share, and mount_tag with an arbitrary string that will be used to identify the shared file system in the guest.

It should now be possible to mount the folder in the shared machine:

# mount -t virtiofs mount_tag /mnt/mount/path
Add the following fstab entry to mount the folder automatically at boot:

/etc/fstab
...
mount_tag /mnt/mount/path virtiofs rw,noatime 0 0
Mapping user/group IDs in unprivileged mode
By default, the root user (id 0) in the guest is mapped to the current user on the host. Other IDs are mapped to the subordinate user IDs specified in subuid(5) and subgid(5).

You can also configure this mapping manually using idmap tag:

# virsh edit name_of_virtual_machine
<domain>
...
  <devices>
    ...
    <filesystem type='mount' accessmode='passthrough'>
      <idmap>
        <uid start="2000" target="1000" count="1"/>
        <gid start="2000" target="1000" count="1"/>
      </idmap>
    </filesystem>
    ...
  </devices>
</domain>
9p
File system directories can be shared using the 9P protocol. Details are available in QEMU's documentation of 9psetup.

Configure the virtual machine as follows:

<domain>
...
  <devices>
    ...
    <filesystem type="mount" accessmode="mapped">
      <source dir="/path/on/host"/>
      <target dir="mount_tag"/>
    </filesystem>
  </devices>
</domain>
Boot the guest and mount the shared directory from it using:

# mount -t 9p -o trans=virtio,version=9p2000.L mount_tag /path/to/mount_point/on/guest
See https://docs.kernel.org/filesystems/9p.html for more mount options.

To mount it at boot, add it to the guest's fstab:

/etc/fstab
...
mount_tag	/path/to/mount_point/on/guest	9p	trans=virtio,version=9p2000.L	0 0
The module for the 9p transport (i.e. 9pnet_virtio for trans=virtio) will not be automatically loaded, so mounting the file system from /etc/fstab will fail and you will encounter an error like 9pnet: Could not find request transport: virtio. The solution is to preload the module during boot:

/etc/modules-load.d/9pnet_virtio.conf
9pnet_virtio
Samba / SMB
An other easy way to share data between guest and host is to use the smb protocol. While performance and latency may not be as good as in the other described ways, its sufficient for simple tasks like transfering simple files like images or documents from and to the guest.

The smb server can be set up directly on either the host, or the guest, for example using Samba, eliminating the need for a dedicated file server. Windows guests have the ability to create smb shares included right after installation (Microsoft Supportpage).

One possible way to access the share under linux (either from the host, or from the guest, depending, where you have installed your server) is to create an entry in your fstab. The samba package is required.

/etc/fstab
#Accessing a samba share on my vm from the host
//my_vm/my_share /home/archuser/my_vm cifs _netdev,noauto,nofail,user,credentials=/home/archuser/.config/my_vm.key,gid=1000,uid=984 0 0
_netdev,noauto,nofail ensures that the share is only mounted when needed without causing issues if the virtual machine is not booted. user,credentials=/home/user/.config/my_vm.key,gid=1000,uid=984 gives you the ability to mount the share on the fly while first accessing it, without needing a password.

UEFI support
Libvirt can support UEFI virtual machines through QEMU and OVMF.

Install the edk2-ovmf package.

Restart libvirtd.

Now you are ready to create a UEFI virtual machine. Create a new virtual machine through virt-manager. When you get to the final page of the New VM wizard, do the following:

Click Customize configuration before install, then select Finish.
In the Overview screen, change the Firmware field to:
UEFI x86_64: /usr/share/edk2/x64/OVMF_CODE.4m.fd for x64 UEFI without Secure Boot support,
UEFI x86_64: /usr/share/edk2/x64/OVMF_CODE.secboot.4m.fd for x64 UEFI with Secure Boot support (without any pre-enrolled certificates).
Click Apply.
Click Begin Installation.
See Fedora:Using UEFI with QEMU for more information.

Tips and tricks
Using an entire physical disk device inside the virtual machine
You may have a second disk with a different OS (like Windows) on it and may want to gain the ability to also boot it inside a virtual machine. Since the disk access is raw, the disk will perform quite well inside the virtual machine.

Windows virtual machine boot prerequisites
Be sure to install the virtio drivers inside the OS on that disk before trying to boot it in the virtual machine. For Win 7 use version 0.1.173-4. Some singular drivers from newer virtio builds may be used on Win 7 but you will have to install them manually via device manager. For Win 10 you can use the latest virtio build.

Set up the windows disk interface drivers
You may get a 0x0000007B bluescreen when trying to boot the virtual machine. This means Windows can not access the drive during the early boot stage because the disk interface driver it would need for that is not loaded / is set to start manually.

The solution is to enable these drivers to start at boot.

In HKEY_LOCAL_MACHINE\System\CurrentControlSet\Services, find the folders aliide, amdide, atapi, cmdide, iastor (may not exist), iastorV, intelide, LSI_SAS, msahci, pciide and viaide. Inside each of those, set all their "start" values to 0 in order to enable them at boot. If your drive is a PCIe NVMe drive, also enable that driver (should it exist).

Find the unique path of your disk
Run ls /dev/disk/by-id/: tere you pick out the ID of the drive you want to insert into the virtual machine, for example ata-TS512GMTS930L_C199211383. Now add that ID to /dev/disk/by-id/ so you get /dev/disk/by-id/ata-TS512GMTS930L_C199211383. That is the unique path to that disk.

Add the disk in QEMU CLI
In QEMU CLI that would probably be -drive file=/dev/disk/by-id/ata-TS512GMTS930L_C199211383,format=raw,media=disk.

Just modify file= to be the unique path of your drive.

Add the disk in libvirt
In libvirt XML that translates to

$ virsh edit vmname
...
    <disk type="block" device="disk">
      <driver name="qemu" type="raw" cache="none" io="native"/>
      <source dev="/dev/disk/by-id/ata-TS512GMTS930L_C199211383"/>
      <target dev="sda" bus="sata"/>
      <address type="drive" controller="0" bus="0" target="0" unit="0"/>
    </disk>
...
Just modify "source dev" to be the unique path of your drive.

Add the disk in virt-manager
When creating a virtual machine, select "import existing drive" and just paste that unique path. If you already have the virtual machine, add a device, storage, then select or create custom storage. Now paste the unique path.

Python connectivity code
The libvirt-python package provides a Python API in /usr/lib/python3.x/site-packages/libvirt.py.

General examples are given in /usr/share/doc/libvirt-python-your_libvirt_version/examples/

Unofficial example using qemu-desktop and openssh:

#! /usr/bin/env python3
import socket
import sys
import libvirt

conn = libvirt.open("qemu+ssh://xxx/system")
print("Trying to find node on xxx")
domains = conn.listDomainsID()
for domainID in domains:
    domConnect = conn.lookupByID(domainID)
    if domConnect.name() == 'xxx-node':
        print("Found shared node on xxx with ID {}".format(domainID))
        domServ = domConnect
        break
Advanced Format 4K native disk
To turn a disk into an Advanced Format 4Kn disk, both its physical and logical sector size needs to be set to 4 KiB. For virtio-blk and virtio-scsi this can be done by setting the logical_block_size and physical_block_size options with the <blockio> element. For example:

# virsh edit name_of_virtual_machine
<domain>
  ...
  <devices>
    ...
    <disk type='file' device='disk'>
      ..
      <blockio logical_block_size='4096' physical_block_size='4096'/>
    </disk>
    ...
  </devices>
</domain>
Commanding QEMU
Libvirt is capable of passing on QEMU command line arguments to the underlying QEMU instance running the VM. This functionality is highly useful when libvirt does not provide QEMU features (yet). For examples, see the entire Intel GVT-g article.

Modify virtual machine XML schema for QEMU
This serves to enable QEMU-specific elements. Change

$ virsh edit vmname
<domain type='kvm'>
to

$ virsh edit vmname
<domain xmlns:qemu='http://libvirt.org/schemas/domain/qemu/1.0' type='kvm'>
QEMU command line arguments
In libvirt, QEMU command line arguments separated by whitespaces need to be provided separately.

The correct location to insert them is at the end of the <domain> element, i. e. right above the closing </domain> tag.

-display gtk,gl=es,zoom-to-fit=off
Becomes

$ virsh edit vmname
...
  </devices>
  <qemu:commandline>
    <qemu:arg value="-display"/>
    <qemu:arg value="gtk,gl=es,zoom-to-fit=off"/>
  </qemu:commandline>
</domain>
Troubleshooting
PulseAudio on system instance
The PulseAudio daemon normally runs under your regular user account, and will only accept connections from the same user. This can be a problem if QEMU is being run as root through libvirt. To run QEMU as a regular user, edit /etc/libvirt/qemu.conf and set the user option to your username.

user = "dave"
You will also need to tell QEMU to use the PulseAudio backend and identify the server to connect to. Add the following section to your domain configuration using virsh edit.

  <audio id="1" type="pulseaudio" serverName="/run/user/1000/pulse/native">
    <input latency="20000"/>
    <output latency="20000"/>
  </audio>
1000 is your user id. Change it if necessary.

You can omit the latency settings (in microseconds) but using the defaults might result in crackling.

Hypervisor CPU use
Default virtual machine configuration generated by virt-manager may cause rather high (10-20%) CPU use caused by the QEMU process. If you plan to run the virtual machine in headless mode, consider removing some of the unnecessary devices.

Virtual machine cannot be un-paused on virt-manager
If you are using a disk image format such as qcow2 which has a specified virtual capacity, but only stores what is needed, then you need to have space on the host partition for the image to grow. If you see I/O related errors when attempting to start the VM, it is possible that the host partition holding the virtual disk image is full. You can run df -h on the host to verify how much free space is available.

If this is the case, see System maintenance#Clean the filesystem for ways to free up space.

Redirect USB Device is greyed out in virt-manager
If the Redirect USB Device menu item is greyed out, check that the following hardware is configured for the VM:

A USB Controller.
One or more USB Redirectors.
Error starting domain: Requested operation is not valid
When you try to open a virtual machine this error may pop up. This is because when you try to open a existing virtual machine libvirt tries to search for the default network which is not available. To make it available you have to autostart your network interface so that whenever your restart your computer your network interface is always active. See libvirt networking page.

Look at the name of your network interface with the following command:

# virsh net-list --all
To autostart your network interface:

# virsh net-autostart name_of_the_network
To start your network interface:

# virsh net-start name_of_the_network
Virt Manager Error 'Virt Manager doesn't have search permissions'
Ensure the folder containing your virtual machine files and installation ISO are owned by the libvirt-qemu group

# chown -R $USER:libvirt-qemu /path/to/virtual/machine
Error starting domain: Requested operation is not valid: network 'default' is not active
If for any reason the default network is deactivated, you will not be able to start any guest virtual machines which are configured to use the network. Your first attempt can be simply trying to start the network with virsh.

# virsh net-start default
For additional troubleshooting steps, see [3]