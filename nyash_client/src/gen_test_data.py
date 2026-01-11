






def main():
    start_num = 0x8adb7b7e8a722df091ecea988a4ad2234836636a102ceb688b3985f89bf40002
    num1 = start_num+1
    num2 = start_num+312
    print(start_num.to_bytes(32).hex())
    print(num1.to_bytes(32).hex())
    print(num2.to_bytes(32).hex())


if __name__ == '__main__':
    main()
