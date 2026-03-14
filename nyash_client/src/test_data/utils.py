import subprocess
import json

def read_metadata(file_name: str) -> dict:
    #cryptsetup luksDump --dump-json-metadata /dev/loop0
    luks_cmd: list[str] = ["cryptsetup", "luksDump", "--dump-json-metadata", file_name]

    result = subprocess.run(luks_cmd, capture_output=True, encoding="UTF-8")
    if result.returncode == 0 and result.stdout is not None:
        metadata = json.loads(result.stdout)
        return metadata
    else:
        raise Exception(f"Error executing 'cryptsetup' binary! {result.stderr}")


def read_encrypted_key(f_name: str, metadata: dict, keyslot: int) -> bytes:
    stripes = metadata["keyslots"][str(keyslot)]["af"]["stripes"]
    offset = int(metadata["keyslots"][str(keyslot)]["area"]["offset"])
    # size = int(metadata["keyslots"][str(keyslot)]["area"]["size"])
    key_size = metadata["keyslots"][str(keyslot)]["area"]["key_size"]
    with open(f_name, 'rb') as luks_file:
        luks_file.seek(offset)
        data = luks_file.read(key_size*stripes)
    return data

