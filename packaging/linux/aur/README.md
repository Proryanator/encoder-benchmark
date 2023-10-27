# Arch User Repository

This directory contains submodules that reference the AUR git repos.

## First Time Setup

If you would like to contribute, please follow these steps:

1. Register for an [AUR Account](https://aur.archlinux.org/register)
1. Provide [e-dong](https://github.com/e-dong?tab=repositories) with your username and ask him to add you as a package co-maintainer
1. Follow instructions for Authentication to get SSH authentication setup in the [AUR Submission Guidelines](https://wiki.archlinux.org/title/AUR_submission_guidelines#authentication) page from the Arch wiki.
1. In your local cloned copy of `encoder-benchmark`, run `git submodule update --init`. The submodules should be initialized and cloned.

## Submodules

- `main` This is the submodule that represents the latest dev version from the main branch.  
  AUR Website: [encoder-benchmark-git](https://aur.archlinux.org/packages/encoder-benchmark-git)
- `release` This is the submodule that represents the latest release tag.  
  AUR Website: [encoder-benchmark](https://aur.archlinux.org/packages/encoder-benchmark)

## References

- https://wiki.archlinux.org/title/Arch_User_Repository
- https://wiki.archlinux.org/title/PKGBUILD
- https://wiki.archlinux.org/title/AUR_submission_guidelines
- https://wiki.archlinux.org/title/Arch_package_guidelines
- https://wiki.archlinux.org/title/Rust_package_guidelines
