# game-data-reader
Displays various data from games. For now, the only value getting tracked is the internal rank (difficulty level).  
It's a lot of effort to go through the game data and ensure the correct values are being tracked, so it's entirely possible that some values are incorrect.  
To use, simply run the program and it will look for bsnes v115 or mame (version support vary per game) running one of the supported games.  

## Supported games:
### Snes (bsnes v115)
```
Gradius III
Parodius Da
```

### Arcade (Mame, see versions below)
```
Ghouls 'n Ghosts | 0.242 - 0.243, 0.246
Gradius II       | 0.246
Gradius III      | 0.242 - 0.243, 0.246
Super Pang       | 0.246
```

### todo / goals
- [x] remember positions and sizes of egui windows  
- [x] allow window resizing  
- [ ] add some cool example screenshot  
- [ ] support for gradius 1 arcade  
- [ ] support for smash tv snes (enemy type/count)  