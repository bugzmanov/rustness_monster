# NES Emulator

<p align="center">
  <img src="https://user-images.githubusercontent.com/502482/94373430-766ebb80-00d3-11eb-82c0-753be5e8b3ef.png" alt="" width="20%">
  <img src="https://user-images.githubusercontent.com/502482/94373466-c51c5580-00d3-11eb-8547-37fc0351e0c7.png" alt="" width="20%">
  <img src="https://user-images.githubusercontent.com/502482/94373510-e67d4180-00d3-11eb-9c38-9ca76cbba062.png" alt="" width="20%">
  <img src="https://user-images.githubusercontent.com/502482/94373543-001e8900-00d4-11eb-8bf4-3e5c1ab3d25e.png" alt="" width="20%">
</p>

#### *My tiny COVID-19 project*

This is yet another emulator of NES platform written in rust. 
The project is far from being complete, but it can play first-gen NES games, including platformers.

I've tried it with:
* Super Mario Bros (horizontal scroll)
* Ice climber (vertical scroll)
* Popeye
* Baloon fight
* Donkey Kong
* Pacman

I also wrote a mini ebook on this topic. Check it out.
<!-- <p align="center"> -->
<a target="_blank" href="https://bugzmanov.github.io/nes_ebook/index.html"><img src="https://bugzmanov.github.io/nes_ebook/images/intro.png" width="20%"/>
<!-- </p> -->

## Running instructions

### Prerequisites
1) rustc
2) cargo
3) git  

### Installation

Macos:
```
brew install sdl2
git clone git@github.com:bugzmanov/rustness_monster.git
```

Linux:
1) install sdl2: http://lazyfoo.net/tutorials/SDL/01_hello_SDL/linux/index.php
2) Clone repo

### Running the game

```
cargo run --release -p native <path_to_rom>
```

### Control
* Keyboard: 
    | Control | Keyboard | 
   | ----------- | ---------- | 
    | Arrows | Arrows | 
    | A,B | a, s | 
    | Start | Enter | 
    | Select | Space | 

* Joystick
    * Assumes joytick based controll if joystick is connected upon emulator start


## Plan

- [x] CPU
- [x] ROM  
  -   [x] Basic support
  -   [x] Mapper 0
  -   [ ] Mapper 1
- [x] Bus, Interrupts
- [x] PPU
 -    [x] Registers
 -    [x] DMA
 -    [x] Rendering
 -    [x] Scorlling
 -    [50%] Sprite 0
- [x] Controllers
 -    [x] Keyboard
 -    [x] Joystick
- [ ] APU