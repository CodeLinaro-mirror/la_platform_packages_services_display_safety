### Instructions to initialize Display Safety PDK

#### Prerequisites
[Set up your build environment](https://source.android.com/docs/setup/build/building#initialize). This is required to be able to use `repo` and the `${T}` variable referenced below.

1. Create a `local_manifests` directory under `<repo_root>/.repo/` if it doesn't exist - `mkdir -p
   ${T}/.repo/local_manifests`
2. Copy `display-safety.xml` to `<repo root>/.repo/local_manifests` - `cp
   ${T}/vendor/google/display_safety/service/pdk/display-safety.xml ${T}/.repo/local_manifests`
3. Sync your repository by running `repo sync`
