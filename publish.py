import json

import platform
import datetime
import tempfile

import subprocess


def rename_arch(arch):
    if arch == "arm":
        return "aarch64"
    else:
        return arch
    
def get_release_path(platform_name):
    if platform_name == "darwin":
        return "target/release/bundle/macos"

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

    with open(release_path + "/Open Tab.app.tar.gz.sig") as f:
        signature = f.read()

    info = {
      "version": f"v{version}",
      "notes": "Test version",
      "pub_date": date,
      "platforms": {
        f"{platform_name}-{arch}": {
          "signature": signature,
          "url": f"https://releases.debateresult.com/autoupdate/v{version}/app-{platform_name}-{arch}.app.tar.gz"
        },
      }
    }

    # make temp dir
    with tempfile.TemporaryDirectory() as temp_dir:
        info_json = json.dump(info, open(f"{temp_dir}/info.json", "w"))
        # run scp to the server
        # scp info.json to the server

        process = subprocess.Popen(["ssh", "-p", "7822", "debatere@nl1-ss6.a2hosting.com", f"''mkdir -p ./releases.debateresult.com/autoupdate/v{version}''"])
        process.wait()
        
        process = subprocess.Popen(['scp', "-P", "7822", f"{temp_dir}/info.json", f"debatere@nl1-ss6.a2hosting.com:releases.debateresult.com/{platform_name}_{arch}"],
                     stdout=subprocess.PIPE, 
                     stderr=subprocess.PIPE)
        process.wait()

        process = subprocess.Popen(['scp', "-P", "7822", f"{release_path}/Open Tab.app.tar.gz", f"debatere@nl1-ss6.a2hosting.com:releases.debateresult.com/autoupdate/v{version}/app-{platform_name}-{arch}.app.tar.gz"],
                     stdout=subprocess.PIPE, 
                     stderr=subprocess.PIPE)
        process.wait()




    # scp to the server
