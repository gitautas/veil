
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
    - 0x00: EV_SYN 
    - 0x01: EV_KEY
    - 0x02: EV_REL
    - 0x03: EV_ABS
    - 0x04: EV_MSC
    - 0x05: EV_SW
    - 0x11: EV_LED
    - 0x12: EV_SND
    - 0x14: EV_REP
    - 0x15: EV_FF
    - 0x16: EV_PWR
    - 0x17: EV_FF_STATUS
    - 0x1f: EV_MAX
    - 0x20: EV_CNT

IEC: The input event code to send, refer to input-event-codes.h for a full list, the important ones for gaming are as follows:
    - 0x00: ABS_X          
    - 0x01: ABS_Y          
    - 0x03: ABS_RX         
    - 0x04: ABS_RY         

    - 0x130: BTN_SOUTH      
    - 0x131: BTN_EAST       
    - 0x134: BTN_WEST       
    - 0x133: BTN_NORTH      
    - 
    - 0x136: BTN_TL         
    - 0x137: BTN_TR         
    - 0x138: BTN_TL2        
    - 0x139: BTN_TR2        

    - 0x13a: BTN_SELECT     
    - 0x13b: BTN_START      
    - 
    - 0x13d: BTN_THUMBL     
    - 0x13e: BTN_THUMBR     
    - 
    - 0x220: BTN_DPAD_UP    
    - 0x221: BTN_DPAD_DOWN  
    - 0x222: BTN_DPAD_LEFT  
    - 0x223: BTN_DPAD_RIGHT 

    - 0x13c: BTN_MODE       

Value: The float32 axis value for the ABS event, in this case used for gamepad axes.

