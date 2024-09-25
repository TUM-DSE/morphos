setup:
    sudo ip link set dev clicknet down 2> /dev/null
    sudo ip link del dev clicknet 2> /dev/null
    sudo ip link add dev clicknet type bridge
    sudo ip address add 172.44.0.1/24 dev clicknet
    sudo ip link set dev clicknet up

    sudo ip link set dev controlnet down 2> /dev/null
    sudo ip link del dev controlnet 2> /dev/null
    sudo ip link add dev controlnet type bridge
    sudo ip address add 173.44.0.1/24 dev controlnet
    sudo ip link set dev controlnet up