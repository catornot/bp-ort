# bp-ort
(bots plugin or other)

the main feature of this plugin is ofc the bots which I have a lot on and yet they still are not great. 
The bots are good enough tho, They work most of the time and in most situations.
Crashes can still happen but are much rarer now which is good :)

most concommands in this plugin have completions to make it easier to use them

Other features that this plugin offers
- auto mp_box loading
- disabling the limit on players in sp maybe?
- bringing back a less extensive version of r_drawworld
- name uwufication (by default)
- name changing (maybe sq api at some point)
- "other testing functionality"
- admin abuse

### extras
- if you have any suggestions pls do tell me
- if you want to add new bots ai pls contact me since it's not documented well
- this will be a pre-release for now since I am simply tired of working on this for ~1.5 years

# bots
they are mostly implemented in the plugin exepected fetching the titan class which requries guidence from scripts.
To add new functionallity a simple api could be used to change the behavior of the bots at runtime from scripts or with cvar.

### current sq api
uh just look at the source code or ask me (@catornot)

### bot names
so bots have "unique" names either derived from contributors to n* to make bot puns or from rust

if you have a good name idea you can make a pull request

bot names can also be provided when spawning

### bot ai

the two most useful ones would be simply standing still (0) (for testing stuff) or the combat ai (6)

the combat ai is currently very not very smart since it just chases the closest enemy and tries to kill them

### cmds

- `bot_spawn <name:int> <team:int> <ai:int>`
spawns a bot with a given name team or ai index

other ones are found under `bot_` namespace


### all the ai indices
- 0 => stand still
- 1 => crouch rapidly
- 2 => walk around mp_box
- 3 => chase player0
- 4 => shoot at closest enemy
- 5 => shoot at closest enemy + walk to them
- 6 => "combat ai" (requires navmesh)
- 7 => goal follower assigned from scripts (reqires navmesh)
- 8 => reserved
- 9 => reserved
- 10 => reserved
- 10 => reserved
- 10 => reserved
- 10 => reserved
- 11 => reserved
- 12 => slow crouching 
- 13 => follows farthest player (requires navmesh) 
- 14 => follows closest player (requires navmesh)  
- 15 => smth silly idk #1 
- 16 => smth silly idk #2 
- 17 => view debugger
- 18 => battery yoinker

# other features

## admin abuse (incomplete) with auto completion!!!
some of the commands from script admin abuse are included

## name renaming
contact cat_or_not for this
