local packages = {
    "docker",
    "docker-compose",
    "portainer-bin",
}

return {
    description = "Homelab server tools and container management",
    post_install_hook = "scripts/setup-immich.sh",
    hook_behavior = "skip",
    packages = packages,
}
