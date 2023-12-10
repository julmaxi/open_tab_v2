import json

import platform
import datetime
import tempfile

import subprocess


def rename_arch(arch):
    if arch == "arm":
        return "aarch64"
    elif "amd64" in arch or "x86_64" in arch:
        return "x86_64"
    else:
        return arch
    
def get_release_path(platform_name):
    if platform_name == "darwin":
        return "target/release/bundle/macos"
    elif platform_name == "windows":
        return "target/release/bundle/msi"
    elif platform_name == "linux":
        return "target/release/bundle/appimage"


def get_signature_name(platform_name, version):
    if platform_name == "darwin":
        return "Open Tab.app.tar.gz.sig"
    elif platform_name == "windows":
        return f"Open Tab_{version}_x64_en-US.msi.zip.sig"
    elif platform_name == "linux":
        return f"open-tab_{version}_amd64.AppImage.tar.gz.sig"

def get_bundle_name(platform_name, version):
    if platform_name == "darwin":
        return "Open Tab.app.tar.gz"
    elif platform_name == "windows":
        return f"Open Tab_{version}_x64_en-US.msi.zip"
    elif platform == "linux":
        return f"open-tab_{version}_amd64.AppImage.tar.gz"

def get_bundle_target_name(platform_name, arch):
    if platform_name == "darwin":
        return f"app-{platform_name}-{arch}.app.tar.gz"
    elif platform_name == "windows":
        return f"app-{platform_name}-{arch}.msi.zip"
    elif platform == "linux":
        return f"app-{platform_name}-{arch}_amd64.AppImage.tar.gz"

def get_full_bundle_name(platform_name, arch, version):
    if platform_name == "linux":
        return f"open-tab_{version}_amd64.AppImage"


def get_non_versioned_bundle_path(platform_name, arch):
    return "download/" + platform_name + "_" + arch

def get_non_versioned_bundle_name(platform_name):
    if platform_name == "linux":
        return f"Open_Tab.AppImage"


if __name__ == "__main__":
    platform_name = platform.system().lower()
    arch = platform.processor().lower()
    arch = rename_arch(arch)

    with open("open_tab_app/src-tauri/tauri.conf.json", "r") as f:
        tauri_config = json.load(f)
      
    version = tauri_config["package"]["version"]
    now = datetime.datetime.now()
    date = now.strftime("%Y-%m-%dT%H:%M:%SZ")

    print(platform_name, arch, version, date)

    release_path = get_release_path(platform_name)

    with open(release_path + "/" + get_signature_name(platform_name, version)) as f:
        signature = f.read()
    
    target_name = get_bundle_target_name(platform_name, arch)

    info = {
      "version": f"v{version}",
      "notes": "Update Opt-Out",
      "pub_date": date,
      "platforms": {
        f"{platform_name}-{arch}": {
          "signature": signature,
          "url": f"https://releases.debateresult.com/autoupdate/v{version}/{target_name}"
        },
      }
    }

    # make temp dir
    with tempfile.TemporaryDirectory() as temp_dir:
        info_json = json.dump(info, open(f"{temp_dir}/info.json", "w"))
        # run scp to the server
        # scp info.json to the server
        print(arch)

        process = subprocess.Popen(["ssh", "-p", "7822", "debatere@nl1-ss6.a2hosting.com", f"''mkdir -p ./releases.debateresult.com/autoupdate/v{version}''"])
        process.wait()
        process = subprocess.Popen(["ssh", "-p", "7822", "debatere@nl1-ss6.a2hosting.com", f"''mkdir -p ./releases.debateresult.com/release/v{version}''"])
        process.wait()
        process = subprocess.Popen(["ssh", "-p", "7822", "debatere@nl1-ss6.a2hosting.com", f"''mkdir -p ./releases.debateresult.com/{get_non_versioned_bundle_path(platform_name, arch)}''"])
        process.wait()

        process = subprocess.Popen(['scp', "-P", "7822", f"{temp_dir}/info.json", f"debatere@nl1-ss6.a2hosting.com:releases.debateresult.com/{platform_name}_{arch}"],
                     stdout=subprocess.PIPE, 
                     stderr=subprocess.PIPE)
        process.wait()

        process = subprocess.Popen(['scp', "-P", "7822", f"{release_path}/{get_bundle_name(platform_name, version)}", f"debatere@nl1-ss6.a2hosting.com:releases.debateresult.com/autoupdate/v{version}/{target_name}"],
                     stdout=subprocess.PIPE, 
                     stderr=subprocess.PIPE)
        process.wait()

        full_bundle_name = get_full_bundle_name(platform_name, arch, version)
        process = subprocess.Popen(['scp', "-P", "7822", f"{release_path}/{full_bundle_name}", f"debatere@nl1-ss6.a2hosting.com:releases.debateresult.com/release/v{version}"],
                     stdout=subprocess.PIPE, 
                     stderr=subprocess.PIPE)
        process.wait()

        process = subprocess.Popen(["ssh", "-p", "7822", "debatere@nl1-ss6.a2hosting.com", f"''ln -f -s '/home/debatere/releases.debateresult.com/release/v{version}/{full_bundle_name}' './releases.debateresult.com/{get_non_versioned_bundle_path(platform_name, arch)}/{get_non_versioned_bundle_name(platform_name)}'''"])
        process.wait()




    # scp to the server
