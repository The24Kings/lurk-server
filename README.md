# Server/ Client for a text based Dungeon Crawler game
```
                                       /@
                       __        __   /\/
                      /==\      /  \_/\/
                    /======\    \/\__ \__
                  /==/\  /\==\    /\_|__ \
               /==/    ||    \=\ / / / /_/
             /=/    /\ || /\   \=\/ /
          /===/   /   \||/   \   \===\
        /===/   /_________________ \===\
     /====/   / |                /  \====\
   /====/   /   |  _________    /  \   \===\    THE LEGEND OF
   /==/   /     | /   /  \ / / /  __________\_____      ______       ___
  |===| /       |/   /____/ / /   \   _____ |\   /      \   _ \      \  \
   \==\             /\   / / /     | |  /= \| | |        | | \ \     / _ \
   \===\__    \    /  \ / / /   /  | | /===/  | |        | |  \ \   / / \ \
     \==\ \    \\ /____/   /_\ //  | |_____/| | |        | |   | | / /___\ \
     \===\ \   \\\\\\\/   /////// /|  _____ | | |        | |   | | |  ___  |
       \==\/     \\\\/ / //////   \| |/==/ \| | |        | |   | | | /   \ |
       \==\     _ \\/ / /////    _ | |==/     | |        | |  / /  | |   | |
         \==\  / \ / / ///      /|\| |_____/| | |_____/| | |_/ /   | |   | |
         \==\ /   / / /________/ |/_________|/_________|/_____/   /___\ /___\
           \==\  /               | /==/
           \=\  /________________|/=/         ______
             \==\     _____     /==/         / _____)
            / \===\   \   /   /===/         ( (____  _____  ____ _   _ _____  ____
           / / /\===\  \_/  /===/            \____ \| ___ |/ ___) | | | ___ |/ ___)
          / / /   \====\ /====/              _____) ) ____| |    \ v /| ____| |
         / / /      \===|===/               (______/|_____)_|     \_/ |_____)_|
         |/_/         \===/
                        =
```

## [Lurk Server Protocol](https://isoptera.lcsc.edu/~seth/cs435/lurk_2.3.html) create by S. Seth Long, Ph.D

The Lurk protocol is intended to support text-based MMORPG-style games, also known as MUDs (Multi-User Dimension). 

It consists of 14 types of message, some of which are primarily sent by servers and some by clients. Behavior and game rules are primarily defined by the server, and clients should expect that their character may be updated with different health, location, and wealth at any time. 

The server is responsible for all computation related to game rules, results of battles, or collecting gold. The client is responsible for communicating with the server and interacting with the player.

## Protocol Messages
All protocol message begin with an 8-bit type, followed by 0 or more bytes of further information. The amount of bytes to be read can be determined by the type, and in some cases a message length field further into the message. 

Notes:
- Variable-length text fields are sent without a null terminator. This doesn't include fixed-length text fields like room and player names. Fixed-length text fields must be null-terminated unless they are exactly the maximum size.

- All numbers are sent little-endian. This makes things easy for x86 users at the expense of being unusual.
Except as noted (health), all integer fields are unsigned.