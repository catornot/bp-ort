# bp-ort
(bots plugin or other)

the main feature of this plugin is ofc the bots which I have a lot on and yet they still are not great. 
The bots are good enough tho, They work most of the time and in most situations.
Crashes can still happen but are much rarer now which is good :)

**release date**: before titanfall 3 

most concommands in this plugin have completions to make it easier to use them

Other features that this plugin offers
- auto mp_box loading
- disabling the limit on players in sp maybe?
- bringing back a less extensive version of r_drawworld
- name uwufication
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

### bots

- `void function BotSetTitan(entity bot, string titan)`
- `void function BotSetTargetPos(entity bot, vector target)`
- `void function BotSetSimulationType(entity bot, int sim_type)`
- `int ornull function BotSpawn(string bot_name)`
- `void function AddBotName(string bot_name)`
- `void function ClearBotNames()`

### navigation

- `var ornull function NavigationCreate(int hull)`
- `void function NavigationFindPath(var nav, vector start, vector end)`
- `array<vector> function NavigationGetAllPoints(var nav)`
- `vector ornull  function NavigationNextPoint(var nav)`

there are probably more

### bot names
so bots have "unique" names either derived from contributors to n* to make bot puns or from rust

if you have a good name idea you can make a pull request

bot names can also be provided when spawning

### bot ai

the two most useful ones would be simply standing still (0) (for testing stuff) or the combat ai (6)

the combat ai is currently very not very smart since it just chases the closest enemy and tries to kill them

- `bot_cmds_type <index:int>`
controls which behavior is the default for the bots

- `bot_clang_tag <tag:string>`
the clan tag the bots get on spawn (default is BOT) 

### cmds

- `bot_spawn <name:int> <team:int> <ai:int>`
spawns a bot with a given name team or ai index

other ones are found under `bot_` namespace (they are not so important)

### all the ai indices
- 0 => stand still
- 1 => crouch rapidly
- 2 => walk around mp_box
- 3 => chase player0
- 4 => shoot at closest enemy
- 5 => shoot at closest enemy + walk to them
- 6 => "combat ai" (requires navmesh)
- 7 => goal follower assigned from scripts (reqires navmesh)
- 8 => headhunter ai (requires navmesh)
- 9 => ctf ai (requires navmesh)
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

### comments on "combat ai" and it's derivatives
it's a general purpose routine for the bots to follow.
it's just shoot anything or walk to the closest enemy.
it does support all facets of titanfall 2 gameplay (titan calling, embarking titans, using titans, pilot combat, etc) but it's very basic.
it also has a feature to actually make them a bit fair where they will get more aim spread the faster the target moves (only for pilots).
the derivations like the headhunter ai try to play the objective of the gamemode ~~but they are not auto activated and have to be manually set via the `bot_cmds_type` cvar~~
auto activation is controlled is controlled by `auto_select_gamemode` in the mod.

# other features

## admin abuse (incomplete) with auto completion!!!
some of the commands from script admin abuse are included in this plugin

## name overriding
Name overrides currently only affects players after a map reload or if it's overridden when they join

**Code Callback** - the plugin attemps to call the following functions when the player connects which are not defined anywhere (aka you can define them)

- `string function CodeCallBack_CanChangeName(string name)`
- `string function CodeCallBack_CanChangeClangTag(string clan_tag)`

**API**

- `void function RememberNameOverride(entity player, string name string clan_tag)`

the plugin stores a name and clan tag for a player internally and will set them for the player on their next connection attempt

- `void function RememberNameOverrideUid(string uid, string name string clan_tag)`

like the one before it but it now accepts a uid so that it can be set before anyone joins

btw the cvar `bot_uwufy` controls if connecting players will get their name uwufied (it's disabled by default now)

# BotExtras
this a optinal but recommend to have script mod for this plugin. it adds extra features on top of the plugin that are simply easier to implement in scripts.

it adds mod settings integration for bp_ort

## exposed functions
- `void function SpawnNBots(int n, string name = "")`
this spawns the specified amount of bots (useful for filling whole lobbies)

usage:
```bash
sv_cheats 1
script thread SpawnNBots(32)
```
