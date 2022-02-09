
# Table of Contents

1.  [VIP - Veil Input Protocol](#org7ff4f35)
    1.  [About](#orgeeebb16)
    2.  [Specification](#org857a368)


<a id="org7ff4f35"></a>

# VIP - Veil Input Protocol


<a id="orgeeebb16"></a>

## About

This is a specification of VIP, the Veil Input Protocol, a network protocol to send Linux input event codes over the network, the intended use of this protocol is cloud gaming and VNC applications.
The reference implementation is used to transport input data over a WebRTC data channel.


<a id="org857a368"></a>

## Specification
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |            EV Type            |              IEC              |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                             Value                             |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

EV Type: specifices the type of input event, the values can be as follows:
    0x00: EV<sub>SYN</sub>
    0x01: EV<sub>KEY</sub>
    0x02: EV<sub>REL</sub>
    0x03: EV<sub>ABS</sub>
    0x04: EV<sub>MSC</sub>
    0x05: EV<sub>SW</sub>
    0x11: EV<sub>LED</sub>
    0x12: EV<sub>SND</sub>
    0x14: EV<sub>REP</sub>
    0x15: EV<sub>FF</sub>
    0x16: EV<sub>PWR</sub>
    0x17: EV<sub>FF</sub><sub>STATUS</sub>
    0x1f: EV<sub>MAX</sub>
    0x20: EV<sub>CNT</sub>

IEC: The input event code to send, refer to input-event-codes.h for a full list, the important ones for gaming are as follows:
    0x00: ABS<sub>X</sub>          
    0x01: ABS<sub>Y</sub>          
    0x03: ABS<sub>RX</sub>         
    0x04: ABS<sub>RY</sub>         

0x130: BTN<sub>SOUTH</sub>      
0x131: BTN<sub>EAST</sub>       
0x134: BTN<sub>WEST</sub>       
0x133: BTN<sub>NORTH</sub>      

0x136: BTN<sub>TL</sub>         
0x137: BTN<sub>TR</sub>         
0x138: BTN<sub>TL2</sub>        
0x139: BTN<sub>TR2</sub>        

0x13a: BTN<sub>SELECT</sub>     
0x13b: BTN<sub>START</sub>      

0x13d: BTN<sub>THUMBL</sub>     
0x13e: BTN<sub>THUMBR</sub>     

0x220: BTN<sub>DPAD</sub><sub>UP</sub>    
0x221: BTN<sub>DPAD</sub><sub>DOWN</sub>  
0x222: BTN<sub>DPAD</sub><sub>LEFT</sub>  
0x223: BTN<sub>DPAD</sub><sub>RIGHT</sub> 

0x13c: BTN<sub>MODE</sub>       

Value: The float32 axis value for the ABS event, in this case used for gamepad axes.

