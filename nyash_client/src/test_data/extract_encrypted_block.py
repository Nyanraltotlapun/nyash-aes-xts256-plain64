from utils import read_metadata


# Init logger

LUKS_FILE_NAME = "vg1-volume_1.img"
KEY_FILE_NAME = "master.key"




def main():

    metadat = read_metadata(LUKS_FILE_NAME)
    print(f"metadata:\n{metadat}")


    segments_offset_bytes = int(metadat["segments"]["0"]["offset"])
    superblock_start_bytes = 0x00010000
    superblock_start_sector = superblock_start_bytes//512
    magic_offset = 0x40
    superblock_lenght_bytes = 0x1000
    sector_size = 512

    with open(LUKS_FILE_NAME, 'rb') as luks_file:
        luks_file.seek(segments_offset_bytes)
        enc_data = luks_file.read(16)


    print("ENC DATA:")
    print("[" + ",".join([format(a, 'd') for a in enc_data])+"]")
    #
    # print()
    # print("KEY DATA:")
    # with open(KEY_FILE_NAME, 'rb') as key_file:
    #     key_data = key_file.read(32)
    #     print("[" + ",".join([format(a, 'd') for a in key_data])+"]")



if __name__ == '__main__':
    main()
