
# Table of Contents

1.  [Veil](#org8a1c9b6)
    1.  [About](#org47ce2e3)
    2.  [Architecture](#org7756eb0)


<a id="org8a1c9b6"></a>

# Veil


<a id="org47ce2e3"></a>

## About

Veil is an open source, self hosted cloud gaming infrastructure that gives you everything you need to run your own cloud gaming application. The main intended use for this is to host for personal use, but it should be relatively simple to add the required functionality to service multiple people as well.


<a id="org7756eb0"></a>

## Architecture

This project is split up into several components, written in different languages and doing different things, the current system is broken down into the following components:

-   [Helgi:](https://github.com/gitautas/veil-helgi) This is the the main WebRTC server written in Go, it does a few things:
    1.  Negotiates and establishes a WebRTC connection with the client
    2.  Packetizes the encoded audio and video frames that are sent by Sigrun and Sif through named pipes and sends them to the client. 
    3.  Streams the packets to the client
    4.  Deserializes the [VIP](./VIP.md) packets sent by the client and replays them using a uinput module
-   [Ymir:](https://github.com/gitautas/veil-ymir) The main REST API written in go, to be replaced by Vlang once either I or someone adds on gRPC functionality.
-   [Sigrun:](https://github.com/gitautas/veil-sigrun) this is the program that is responsible for capturing and encoding a GPU framebuffer. It currently only targets NVidia based systems due to the simplicity of NvFBC and NvENC SDKs, in theory one should be able to replicate this functionality using AMD&rsquo;s AMF framework, but it is considerably worse documented and whether the required APIs are exposed I&rsquo;m still unsure, if I have more time and resources I could research this a bit more, but currently it is not a priority.
-   Sif: This is a shell script? that will manage pipewire loopback devices, encode them into opus and send them over to Helgi over named pipes. This is still TODO.
-   [Kara:](https://github.com/gitautas/veil-kara) This is a vue app that functions as the main client. It negotiates a stream, receives the video and audio streams, reads the gamepad data, serializes it and sends it over to Helgi. This is currently a very minimal implementation written in pure javascript.

This architecture is still very early and is intended to change.

![architecture diagram](veil.svg "architecture diagram")

