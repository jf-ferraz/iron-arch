return {
    description = "GNOME Boxes and virtualization utilities",

    packages = {
        "gnome-boxes",
        "libvirt",
        "virt-manager",
        "dnsmasq",
        "bridge-utils",
    },

    services = {
        enabled = {
            "libvirtd",
        },
        disabled = {},
    },
}
