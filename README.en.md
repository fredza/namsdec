# namsdec

**A workbench for learning x86-64 NASM assembly — and for reading the
binaries other people wrote.**

> Interface in French and English. Linux, ELF64, NASM syntax.

### 🇫🇷 **[Lire en français](README.md)** · 🇬🇧 English

---

namsdec runs your program one instruction at a time and tells you, at
every step, what the machine is about to do and what it just did. It
does not merely display registers: it explains. It is a teaching tool,
MIT licensed, built so that what you understand here can be passed on.

---

## What it shows that the others do not

**The next instruction, resolved.** `mov al, [rsi + rcx*4 + 8]` does not
tell you *where* that points: you have to hold three registers in your
head and do the arithmetic. namsdec does it before execution — the
effective address, its label in your own source (`buffer+2`), and the
byte sitting there.

**The last step, with the value from before.** `RCX 0x5 → 0x4` under the
`dec rcx` that did it, and the flags it changed beside it. After a `cmp`
the flags *are* the result: the instruction touches no register at all.

**The conditional jump, before it goes.** "The jump will be taken", the
rule it follows (`CF = 0 and ZF = 0`), and whether the comparison reads
its numbers as signed. `ja` and `jg` both say "greater" and answer the
opposite on the same bytes: that is exactly where people get it wrong.

**And, for the learner, a way to make it stick: predict before you
step.** The three paragraphs above are an answer handed over before the
question was asked — you press *Step* to confirm what you just read, and
never get the chance to be wrong. Being wrong is the only way to find
out you had not understood. In beginner mode the address and the
instruction stay on screen, the answer waits for the step, and *Last
step* reveals it **next to what actually changed**:

```
Next instruction
  0x400005  mov ebx, 2
  What is this going to change?   [Show]

Last step  0x400000  mov eax, 1
  What it said: EAX = 1
  RAX 0x0 -> 0x1
```

*Show* gives the answer straight away, one click, no confession. The
setting lives in Preferences ▸ Modules, under beginner mode — it only
makes sense there: an exercise forced on somebody who uses the tool
daily would be a tax.

**The system call written out.** `syscall` carries no operand at all —
everything is in the registers, by a convention you either know or go
looking for somewhere else:

```
write(fd = 0x1, buf = 0x402000 → "hello\n", count = 0x6)
```

**The stack read as a stack.** One eight-byte word per line, RSP and RBP
marked, and any value pointing into the code named for what it is: a
return address, with its label. What sits above RSP is greyed out —
those bytes are still there, but the next `push` overwrites them.

**Both shortcut sets, side by side.** Preferences ▸ Shortcuts shows the
two of them, action by action: the active one in full, the other to
compare against, "same" where they do not differ. That is how you see
what you came for — the gaps, what you moved yourself, and two actions
that would answer the same keystroke.

**And you can travel.** Addresses in the analysis panels are clickable:
a relocation entry, a slack region, a shape, a gadget — one click brings
the listing onto it. RIP tracking stops on the way, otherwise the next
step would drag the listing back to where execution is.

**When the listing lies.** A disassembler reads *linearly*; the
processor goes **where the jumps send it**. One trap byte behind a `jmp`
that steps over it, and the whole line-by-line reading goes askew —
classic anti-disassembly, and you meet it in the very things namsdec is
built to read. The **Code shapes** panel gives you **both readings side
by side** and the byte that separates them. What you learn on the way is
worth more than the finding itself: where instructions begin is not a
property of the bytes, it is a property of where you start reading.

**What stands out, and why.** The analysis window opens on **Signals**:
a fooled listing, a segment both writable *and* executable, rising
entropy, a packer recognised by name, anti-debugging, missing
protections, flags hiding in strings. Nothing new is computed there — it
is what the other panels already found, gathered to say where to start.
Each line gives a **fact** (an address, a name, a measurement), **what
that fact changes**, and **the panel** that explains it. Nothing is
added up into a score: a score would look like a verdict, and it is the
reader who concludes.

**What the binary can do, in six families.** Imports and system calls
are listed everywhere; here they are also *sorted* — network, execution,
anti-debugging, memory, cryptography, files. The question is not "which
functions" but "what does this binary do", and three lit families are
already a portrait. A `socket` is a `socket` whether it arrives through
the relocation table or as number 41 in RAX — and a program trying to go
unnoticed uses precisely the second.

**And namsdec can go and fetch the tools.** Rizin + rz-ghidra builds,
RetDec downloads: Preferences ▸ Modules ▸ *Fetch the tools and build
them*, then **Install…**. That module is off by default, and more
plainly so than the others — the others display, this one downloads code
and runs it. So the window shows **the whole recipe before anything
starts**, and the log is written live. No step calls `sudo`, no step
goes through a shell, and nothing is placed outside
`~/.local/share/namsdec/tools/`. The binary does not grow from it:
`curl` downloads, `tar` extracts, `meson` and `cmake` build — just as
`nasm` and `ld` already assemble.

**A decompiler, if you have one.** The **Decompilation** module
(Preferences ▸ Modules, off by default) runs the tool of your choice.
Six are known in advance — `objdump`, `ndisasm`, radare2's pseudo-code,
**Rizin + rz-ghidra**, **RetDec** and **DZF** — with the arguments that
work.

Each card answers **two questions**, and you need both for the tool to
be of any use: is the program **installed on this machine**, and is it
**enabled in namsdec**? The two are independent — a tool installed but
not enabled will not be launched, and a tool enabled that has vanished
from the machine will fail at launch, which the card says in red.
Enabling one **writes a file of a few lines**: namsdec downloads nothing
and launches nothing of its own accord.

The card of a missing tool says **how to get it without unfolding
anything**: that is a breakdown, not a detail. The *Details* fold keeps
only the exact command that will be run. A tool that answers offers only
**Close**: restarting twenty minutes of compilation for an identical
result is not an offer. An **incomplete** tool offers **Repair** — Rizin
without `rz-ghidra`, where `pdg` does not exist and nothing said what to
do about it.

**DZF is in the list although it is still being written**: it is the
in-house x86-64 disassembler, C++23 and no dependencies at all. It does
not read ELF and does not have to learn how — namsdec hands it the
executable bytes and their address. Until it is built, its card says
where to get it. Yours is added the same way, by a JSON file dropped in
`~/.local/share/namsdec/modules/` — with `{out}` in the arguments if the
tool writes its result to a file rather than to standard output. Three
locks, because this means running a program on your machine: the module
is enabled by hand, the window opens on request, and **the command is
spelled out in full before it is run**. The work happens alongside — the
interface does not freeze — and a tool that has not come back after two
minutes is stopped rather than waited on forever. Progress is readable
in the status bar **and in the window title**, so in your system's task
bar: "Namsdec — Decompiling 45%".

**A disassembly panel, with a choice of engine.** Pseudo-code first —
`RAX = RAX + RCX` rather than `add rax, rcx`, `if different, go to loop`
rather than `jne` — then NASM syntax, then the tools you install
yourself: a JSON file in `~/.local/share/namsdec/modules/` describes the
command to run (`objdump`, `ndisasm`, yours). Those are **programs**:
they run only on request, and the panel announces what it is going to
launch before it does.

**Where this value came from.** Hovering a register says which step
wrote it last, its rank and its line: "written at step -12, line 7: add
rax, rcx". Finding that by hand means reading your program backwards,
and getting it wrong every other time as soon as there is a loop. When
there is no history — a binary calling an absent library does not run —
namsdec walks the **code** backwards to the source instead, stopping at
the first jump: beyond it several paths lead here, and "the last write"
no longer has a single meaning.

**Icon rails at the edges.** The explorer and the exercises on the left,
the console at the bottom left, the processor panel on the right — each
icon on the side and at the height where its panel opens. Hovered, it
says what a click will do: "Show the explorer", "Hide the explorer",
with the shortcut as you have set it. They command nothing the View menu
does not command already; what they add is making visible what exists —
a closed panel left no trace on screen, and you do not close what you do
not know how to reopen. Each panel also carries a dash at the end of its
tab row, which puts it away. The console carries its three commands
there — restart, stop, clear — next to what they change. Clearing is not
resetting: it is the screen you empty, and the processor does not move
by one instruction.

**The count of problems, without opening the list.** At the right edge
of the view row: a red disc and the number of errors, an orange triangle
and the number of warnings. A click opens the list on the right tab; the
two chevrons lead to the previous and the next problem, wrapping around.
It only appears if there is something to count — a counter permanently
at zero is a place on the screen you learn to stop looking at.

**The view row fits on one line.** What does not fit goes behind a
button that says how many — `+5` — and the view you are looking at
always stays visible. The row used to wrap: at 800×520 the ten views
took three lines, ninety-six pixels, enough to paint six lines of code —
in order to show ten tabs of which you look at one. Four lines of
program were left on screen.

**Several files open at once.** A tab bar above the editor, the name
shortened in the middle when it is long — so that `loop_i…ee.asm` and
`loop_i…e_2.asm` stay two different files — and a dot on what is not
saved. An open exercise takes its tab too, under its title. Each tab
keeps **everything** you were doing in it: the text typed, the cursor,
the breakpoints, and execution stopped at the step where you left it.
Ctrl+Tab moves to the next, Ctrl+W closes, and the files come back on
the next launch.

**The home screen, once you closed the last one.** The name of the
workshop, then what you start again from — new file, open, command
palette, exercises — and the recent files, which a click reopens. Every
shortcut shown is the one from the shortcut table, read and not copied.
It is **asked for** by a gesture: closing. It fits on screen. It is not
deduced from an empty buffer, otherwise it would appear in the middle of
typing — you select all, you delete to start again, and the editor would
vanish under your fingers.

**The shape of the program, not its sequence of instructions.** `Space
g` opens the **flow graph**: each box is a block you can only enter from
the top and leave from the bottom, arrows show the taken jump (green),
the untaken one (red) and the call (blue), and a loop shows up by going
back up. The block the program is in is brought forward, a click takes
you there in the editor, and Ctrl+wheel zooms out to see the overall
shape. That is what you reconstruct in your head while reading a listing
— and what a beginner cannot see yet.

**Where the program spends its time.** A heat band in the gutter — green
for a line walked once, red for the hottest, yellow and orange in
between — with the exact count on hover and a legend under the editor.
The console's **Profile** tab turns it into a ranking: the number of
passes, the share of the total, the line itself, and a click that takes
you there. The band says where it is hot; the ranking says by how much,
and copies out as text.

**What namsdec will not do.** `execve`, `socket`, `connect`: those
system calls are named but not executed — namsdec has neither a second
process nor a network to offer. A program containing one would stop
there without warning: this is said at assembly time, with the line.
Opening a file, on the other hand, **answers**: "not found", which is
the truth, and the program follows its error branch — often the half you
came to see. The console says so the first time.

**The `int3` a program sets on itself.** It installs a `SIGTRAP`
handler, executes `int3`, and looks at whether it was called: under a
debugger it is the debugger that receives the breakpoint. namsdec calls
it, with a real signal frame — so a handler that rewrites the resume
address to jump over a decoy is obeyed. With no handler, execution stops
and says so.

**What nasm does not tell you.** Two mistakes get through assembly
without a word and only show up at run time — or not at all: a program
that nothing stops, which carries on into whatever follows its code, and
a write to a fixed address that lands in a read-only region. namsdec
re-reads the linked program and reports them as warnings, with their
line.

**What to do when it crashes.** The message says "writing to 0x400000 is
forbidden: it is in .text". Underneath: *code and constants are
read-only; what changes during execution goes in `section .data`*.

**Who calls what.** Cross-references record, for each address, who calls
it, who jumps to it and who names it. An address a `call` aims at is a
function, even stripped of its name — and the one called from everywhere
is the one to start reading from.

**How do you even get there?** In the flow graph, a field takes an
address — the one where the program accepts, say — and gives you **the
route to reach it**, with the conditions to satisfy in order: this `jne`
must not be taken, that one must. The blocks along the path are brought
forward in the drawing. It is the shortest route, and namsdec says when
others exist rather than letting you believe in a proof.

**What the code does that is worth noticing.** The **Shapes** module
picks out constants that are really text (`mov ecx, 0x48555648` →
`"HVUH"`), length checks, comparisons disguised as XOR — `a XOR b` is
zero only if the two are equal, so the constant *is* the expected value
— string comparisons and jump tables. Families are sorted **from the
rarest to the most common**: in a binary of sixty thousand instructions,
seventeen hundred zero tests are punctuation, and four XOR comparisons
are a lead.

**Where to start reading.** The **Functions** module recovers `main` in
a stripped binary — the startup code hands its address to
`__libc_start_main`, in RDI — then cuts the code at call targets, with
the number of callers for each. And strings from languages without a
terminating zero (Rust, Go, Zig) are **cut by the code**, where a byte
scan reads only one blob: there the address travels with its length.

**A binary's armour.** PIE, NX, RELRO, stack canary, FORTIFY, IBT and
shadow stack, W+X segments, symbols, libraries, RPATH, and the functions
it borrows — each relocation entry gets its name back, with the number
of calls leading to it — with, for each, what its state actually
changes. On your own program as on a system binary. The report copies
out as Markdown, or saves, to be pasted into a write-up or a course.

**And also:** documentation for 193 instructions, directives and
concepts under **F1** or on hover — with a table of contents,
cross-references and an export of the whole manual as Markdown (Help
menu), because what you learn here must be readable elsewhere —
completion that only offers what is documented, a De Bruijn cyclic
pattern to measure how far an overflow went (identical to pwntools',
hence comparable), a command palette that also reaches your program's
labels (type the name, you are there), a timeline where you click a past
step to return to it (registers, flags and memory with it), registers
and flags you can set by hand ("what if ZF were 1?"), breakpoints — with
a condition (`rcx == 0`), watching a register or an address, calling
convention checks (what a subroutine borrowed without giving back),
memory protected by the real permissions of the sections, a simulated
heap with `brk`, SSE/AVX and the x87 stack.

## Plugins

The [`plugins/`](plugins/) directory holds what can be added to namsdec
without recompiling it, in two families:

- [**Themes**](plugins/themes/) — colours, and nothing else. A theme
  does not execute.
- [**Extra features**](plugins/fonctionnalites/) — the description of a
  **program** to run: `objdump`, `ndisasm`, `radare2`, Rizin +
  rz-ghidra, RetDec. Each with its installation instructions and the
  arguments that work.

The distinction is not a ranking, it is a matter of trust: namsdec never
goes looking for a program on its own, runs one only on an explicit
request, and writes the full command out before running it.

## The guide

A **PDF user guide** ships with every release:
[`docs/guide-namsdec-1.0.2-beta.1-en.pdf`](docs/guide-namsdec-1.0.2-beta.1-en.pdf)
— installing, getting started, every panel, stepping, memory, the
manual, the flow graph, the exercise path, reverse engineering, the
shortcuts and the known limits.

It is **produced from the repository** (`python3 docs/guide.py`): the
version, the number of exercises, the number of documentation cards and
the shortcut table come from the sources, never from a copy that would
age.

## Installing

You need **Rust** (`cargo`), **nasm** and **ld** (binutils):

```sh
# Fedora
sudo dnf install nasm binutils
# Debian, Ubuntu
sudo apt install nasm binutils
```

Then:

```sh
git clone https://github.com/fredza/namsdec.git
cd namsdec
./install.sh
```

`install.sh` compiles, puts the binary in `~/.local/bin`, and the icon
and launcher where the desktop looks for them. Nothing is written
outside `$HOME` and nothing asks for administrator rights.

**Running it again is an update, not a reinstall.** Nothing is removed:
your settings, the exercise packs you added and your breakpoints stay
where they are, and the script says so up front — "updating 0.4.0 to
0.4.1" rather than "installing". Each file is written beside its target
and then renamed, so at no instant is the target missing or incomplete:
that is what the launcher lacked, which `cat >` left empty for a
fraction of a second — enough for GNOME, which prunes favourites whose
`.desktop` it can no longer find, to pull the icon out of the dock while
you were installing.

**And if namsdec is open**, it stays open for the whole job — compiling
takes a few minutes, and waiting in front of an empty screen helps
nobody. The binary is replaced under the window, which does not notice:
it keeps the one it opened. Only at the very end, once everything is in
place, does the script close it cleanly and reopen it on the new
version. `--no-restart` prevents that. A window that refuses to go — a
question on screen, unsaved work — is never forced: the script says so
and stops there.

`./install.sh --help` lists the options. The most useful ones:

| option | what it does |
|---|---|
| `--dry-run`, `-n` | say what would be done, write nothing |
| `--debug` | install the unoptimised build, ready in seconds |
| `--prefix DIR` | where icons, launcher and exercises go |
| `--bin-dir DIR` | where the binary goes |
| `--no-desktop` | no launcher, no icons |
| `--no-dock` | do not pin to the dock (GNOME) |
| `--no-path` | do not touch the shell configuration file |
| `--no-exercises` | do not install the bundled pack |
| `--no-strip` | keep debug symbols (two megabytes more) |
| `--no-build` | do not compile: install the binary already built |
| `--no-restart` | do not close and reopen namsdec if it is running |
| `--quiet`, `-q` | say only what matters |
| `--no-color` | no colours — `NO_COLOR` is honoured too |

**A lighter binary**: `cargo build --release --no-default-features`
drops the update check, and the TLS stack with it — **16.0 → 14.4 MB**
once stripped, ten per cent. That is what a distribution packager wants:
updates are their job, not the application's. The menu then does not
show the entry, rather than offering a button that can do nothing.

`./uninstall.sh` undoes exactly that, and takes `--prefix`, `--bin-dir`,
`--dry-run`, `--quiet` and `--no-color` the same way. `--purge` also
removes your settings and exercise packs — without it they are kept,
because people usually uninstall in order to reinstall.

To run without installing: `cargo run -p namsdec-gui`.

## Getting started

1. **Ctrl+Shift+B** assembles the program (nasm then ld, for real).
2. **Space s i** steps one instruction (`stepi`), **Space n i** steps
   over a `call`, **Space p r** runs, **Space p b** steps one back. The
   first two keep gdb's letters because they are the most hit keys in
   the whole program; the rest of the sequences are sorted by family:
   `p` for what runs the **program**, `/` to search, `m` to **mark**,
   `v` for the **views**, `u` to run **until** something.
3. **At beginner level the editor has no modes**: you click, you type,
   it is written, and the status bar carries no badge. At advanced level
   the editor has three modes, like vim: **NORMAL** (the `hjkl w b e gg
   G` motions, and the `5j` counter), **INSERT** (`i`, Escape to leave)
   and **VISUAL** (`v`: motions extend a selection, `y` copies, `d`
   cuts, Escape gives up).
4. **F1** on a word in the code gives its card; hovering does too.
5. A click in the gutter sets a breakpoint; **Ctrl+F10** runs to the
   line under the cursor.
6. **Ctrl+Shift+P** opens the palette: every command is in there.

The shortcuts live in Preferences ▸ Shortcuts, and can be changed.

**The console is not there at the start, and that is deliberate.**
Before you have assembled anything it would have one thing to say: that
it is empty. It appears by itself as soon as it has something to show —
an error from nasm, or the first byte the program writes. Closed
mid-run, it stays closed; View ▸ Console opens it on request, and the
setting is kept.

### The exercises

**Each of the one hundred and thirty-eight carries its own theory
card.** Not the brief, not the hint: the brief says what is asked, the
hint says where to look when you are stuck, and the theory says the
mechanism the exercise rests on — what a loop is when the processor has
none, why writing `eax` wipes the top of `rax`, what a calling
`convention` is not obliged to guarantee. You read it before trying, and
it will serve ten other steps.

It quotes, between backticks, the words that have a card, and the
interface turns those into **clickable cross-references** — to
instructions as well as to concepts. Nothing is held by hand: a test
checks that every theory, in both languages, quotes at least one card
that really exists, that it does not merely repeat its brief, that none
is written twice, that it carries a **worked example with numbers**, and
that it fits within the reading bounds of its window.

It is written **for a beginner**: one idea per paragraph, short
sentences, an example with real numbers, and the trap at the end. The
worked example was not there at first: of the hundred and thirty-eight
theories, ninety-seven quoted no instruction with its operands and
fifty-one carried no digit at all. The two hundred and seventy-six texts
now each carry one — the starting values, what each instruction does to
them, the number that comes out — taken from the step's own program and
checked against the repository's assembler.

The **Theory** button sits in the gesture row, next to *A hint* and *The
solution* — the order in which you come to them — and it opens **a
window of its own**. It was a fold inside the band at first, and both
states were bad: unfolded it pushed the **goal** below the fold (four
tests said so), folded for that reason it never opened. A window settles
both, and finally gives a teaching text the room it asks for.

Eighteen exercises ask you to **read** rather than to write: the program
is given complete, a question is asked — "what is RAX at the halt?" —
and you answer before running. The verdict does not hand you the right
answer: you get it by running, that is, by doing what the exercise
teaches. You write "32", "0x20", "20h" or "0b100000" — it is the same
answer, and whoever gives it in binary understood the same thing. **And
the answer written in the pack is the one the machine gives**: a test
assembles each of those programs, runs it and checks — an answer written
by hand in a JSON file is a claim; put to the emulator it is a fact.

One hundred and twelve of the hundred and thirty-eight carry a
**checkable goal** (`rax == 0xf`): the processor banner says "reached"
or "not yet", instead of leaving you to verify by eye. What has been
passed once stays passed: a badge in the list, a count at the head of
the pack, and both survive the next launch.

Each measured exercise also carries a **budget**: the number of
instructions the shipped solution executes. Matching it is normal; going
below is the real exercise — it is while looking for how that you learn
`lea`, shifts, or that a five-round loop can be replaced by a formula.
The best score is kept per exercise, a ring marks those under budget,
and a test checks that every announced budget is reachable: promising a
figure no solution attains would be worse than giving none. An installed
pack can carry them too — the `goal` field of the JSON file, and
`question` + `answer` for a reading exercise.

A path of one hundred and thirty-eight exercises ships with the program,
arranged in four levels of rising difficulty:

- **Discovery** (43) — **the program first**: the four lines of the
  frame (`global`, `section`, the label, `hlt`), three of which are not
  orders; RIP and stepping, because a jump is nothing but an address
  written there; `mov`, which copies instead of moving, and whose
  destination is on the left. Then **the basics**: a bit, the weights 1
  to 128, the byte they form, the nibble a hex digit covers, why it is
  sixteen and not ten. Then **the structure of the registers in full**:
  64, 32, 16 and 8 bits, the names inherited from the old eight (`rax`,
  `eax`, `ax`, `al`) and the suffixes of the numbered eight (`r8`,
  `r8d`, `r8w`, `r8b`), `ah` which is not the top, and the fact that
  there is no four-bit slice. Then exchange, zeroing, `add` and `sub`.
  Then **the jump before the loop**. Then loops, `loop`, bits,
  comparisons, flags, sign. Then **two's complement, step by step**:
  built by hand with `not` and `inc`, the bounds of a byte and their
  asymmetry, the step that tips 127, and `movzx` against `movsx`. Then
  `inc` against `add`, `test`, `xchg`/`bswap`, bit masks, constants
  computed at assembly time, rotations, `setcc`, `not` and `neg`, signed
  division, the cost of an immediate. **Ten** of these steps are
  **prediction** exercises, each placed right after the theory it tests.
- **Practice** (33) — shifts, bit counting, multiplication, division,
  memory, operand sizes, byte order, declared data, arrays, matrices,
  searching, address computation, minimum and maximum, parity,
  occurrences, in-place reversal, branchless clamping, powers of two,
  sum and average, byte arrays, overlapping regions (`memmove`), filling
  with `rep stosb`, bit scanning, 128-bit addition, conversion tables,
  alignment and the holes in a struct. Then **reverse engineering**
  begins, before you even read code: a file's signature, what `strings`
  lets you see, position in the file against address in memory. And
  three **prediction** exercises.
- **Structures** (36) — strings, string instructions, string
  comparison, text to number, macros, records and fields, jump tables,
  branchless choice, the stack, calls, the calling convention,
  recursion, Fibonacci, the stack frame, local arrays, sorting, `strlen`
  by `scasb`, word reversal, word counting, a stack that returns,
  arguments beyond six, function pointers, stack alignment,
  concatenation, word extraction, tail recursion, insertion sort, binary
  search, linked lists, transposition. On the **reverse engineering**
  side: the comparison and the jump that depends on it, unrolling a
  computation rather than running it, a switch seen from the
  disassembly. Two **prediction** exercises: what the stack returns
  first, and `ja` which is not `jg`.
- **Mastery** (26) — output, input, the heap, floating point, SIMD,
  decimal and hexadecimal printing, upper-casing, overflow, protections,
  XOR encryption, checksums, a canary placed by hand, vector selection
  by mask, vector sums, truncation against rounding.

**The path is not in the binary**: it is a data file, and yours is added
the same way.

### If you come from pwndbg

namsdec is not a debugger: it simulates the processor instead of driving
one. But the gestures you make in front of a binary are the same, and
here they carry the same names as elsewhere.

| pwndbg / gdb | namsdec |
| --- | --- |
| `telescope` | Memory tab, "Pointers" reading; and every stack word |
| `vmmap` | Memory tab ▸ Memory map |
| `search` | Memory tab ▸ Search memory |
| `patch` | Memory tab ▸ Write here |
| `set $rax = …` | click the register value |
| `setflag ZF 1` | click the flag |
| `cyclic` | Tools ▸ Cyclic pattern (identical to pwntools) |
| `dumpargs` | the banner, in front of a `call` |
| `retaddr` | Stack tab ▸ Return addresses |
| `nextcall`, `nextret`, `nextsyscall` | Tools ▸ Until the next… (`Space u c/r/s`) |
| `context` | Edit ▸ Copy state |
| `checksec` | Analysis window ▸ Protections |
| (what you look at when opening a sample) | Analysis window ▸ Signals |
| `got`, `plt` | Analysis window ▸ Imported functions |
| `nearpc` | the listing itself, RIP in the gutter |
| `stepi`, `nexti`, `reverse-stepi` | `Space s i`, `Space n i`, `Space p b` |

### Three ways to read the same instruction

The disassembly window offers a choice of engine:

| engine | what it shows | example |
|---|---|---|
| Pseudo-code | the instruction said in plain words | `RAX = RBX` |
| NASM | the syntax you write in the editor | `mov rax, rbx` |
| System | the one `gdb`, `objdump` and `as` speak | `mov %rbx, %rax` |

The **System** syntax is a view, not a way of writing: nasm does not
read it. It is there because everything you find outside namsdec is
written in it.

**In beginner mode the row offers only three**: pseudo-code, NASM, and
`objdump` in Intel syntax — the one on every machine, and whose syntax
is the one you have just learnt by writing nasm. Six ways of reading the
same code in front of somebody who knows none of them is not a choice,
it is one more question. An engine chosen at advanced level keeps its
place if you go back to beginner level: you do not take away what
somebody is in the middle of reading.

### Adding a disassembler

The disassembly panel offers two shipped engines — pseudo-code and NASM
syntax — and every one you describe to it. One JSON file per tool, in
`~/.local/share/namsdec/modules/` — every tool lives there, and the
`"kind"` field says whether the card returns instructions
(`"disassembler"`, the default) or pseudo-code (`"decompiler"`).

**If the tool does not read ELF**, two further markers let it off:
namsdec hands it the executable bytes and their address instead.

## The code

A Rust workspace, one crate per responsibility:

| crate | what it does |
|---|---|
| `namsdec-core` | registers, flags, memory and its permissions |
| `namsdec-asm` | calling nasm and ld, reading the ELF they produce |
| `namsdec-disasm` | x86-64 disassembly (iced-x86), NASM syntax |
| `namsdec-emu` | the interpreter: integers, SSE/AVX, x87, system calls |
| `namsdec-re` | reverse engineering: signals, anti-disassembly, protections, cross-references, strings and flags, behaviour families, system calls, packers, ROP gadgets, slack space |
| `namsdec-doc` | the instruction and directive cards |
| `namsdec-exercises` | the exercise packs and their format |
| `namsdec-theme` | the themes, including those installed from disk |
| `namsdec-explorer` | the file explorer |
| `namsdec-update` | looking up the published release, and SemVer comparison |
| `namsdec-gui` | the interface (egui / eframe) |
| `namsdec-cli` | the analysis command: report, JSON, comparing two versions |

The domain crates know nothing of the interface language: their errors
carry an English message and keep their values in structured variants,
which the interface translates.

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
```

The tests do not merely check functions: they assemble and run real
programs with nasm and ld, draw the interface off-screen and read what
was painted. Every program shipped with the application, and every
example in the cards, must assemble, link without a warning, and run to
the end.

### Checking the emulator against a real CPU

Thirteen thousand hand-written lines of semantics cannot be verified by
re-reading them. A flag computed wrongly does not crash: it returns a
wrong answer, silently, and the emulated program takes a branch it
should not have taken. So the emulator is compared against the real
processor, through gdb — the same program, instruction by instruction,
and every divergence named.

### Measuring what the detour can do

The detour refuses what it cannot relocate, and the interface says so
site by site. It does not say **how many** sites it refuses — and a tool
that refused nearly everywhere would be useless without anybody
noticing.

```sh
cargo run --release -p namsdec-re --example taux_detour -- tests/binaires/*
```

The bench plans a detour at **every instruction address** of every
executable range, and counts the successes against the refusals, by
reason and by mnemonic. Nothing is written: `plan_detour` returns a plan
without touching the file, which is precisely what makes measuring it in
bulk possible.

On the project's eleven test binaries — 3.4 million sites — **two out of
three accept a detour**. `rip`-relative access, feared to be blocking
because it is the normal form of a global access, **never** causes a
refusal. The only refusal of principle is the short jump, one site in
eighteen; the rest comes from the available slack, not from the
instructions.

It scans **linearly**, so a data table embedded in code is counted as
code. That is accepted: measuring only named functions would measure
nothing on a stripped binary, which is the case that matters. And it
falls back on the **segments** when section headers are missing —
without that, two of the eleven binaries returned "zero sites".

## What it does not do

- No 32-bit mode, no `int 0x80`.
- **No dynamic linker.** namsdec loads a binary the way the kernel would
  — segments mapped into pages, relocations applied, thread-local
  memory laid down, the process stack written with `argc`, `argv`, the
  environment and the auxiliary vector — but it loads no library. A
  **statically** linked binary therefore runs in full, C library
  included.
- Instead, namsdec **provides** one hundred and seventy functions of its
  own, written in Rust: `printf` and its cousins, `puts`, `fgets`,
  `scanf` and `sscanf`, the string and memory functions — `strtok`
  included, which cuts in place — the **character classes** and
  `<ctype.h>` tables the glibc reads instead of calling, `malloc` and
  its family — which takes its memory from the heap, where you can see
  it — and the system call wrappers. Which ones to add was not guessed:
  the imports of the whole test corpus, minus what was already
  provided, ranked by the number of binaries asking for them. An
  ordinary **dynamic** binary therefore runs too, from prompt to
  verdict; it stops by **naming** the first function it borrows that
  namsdec does not provide. What is provided is provided openly: the
  count is shown at load time, and the full list is
  `namsdec_emu::PROVIDED_FUNCTIONS`.
- Those functions obey the same rules as everything else: the simulated
  memory permissions apply (a `strcpy` into `.rodata` fails, as it would
  on the real machine), each call counts as **one step**, and what it
  writes is undone by *step back*.
- The simulated system calls are the ones that *do* the work — `read`,
  `write`, `brk`, `mmap`, `mprotect`, `exit` — the ones that report
  without changing the world — `getpid`, `getuid`, `geteuid`,
  `sched_yield`, `nanosleep`, `getrandom`, `uname`, `clock_gettime` —
  and those of a C library's startup: `arch_prctl`, `set_tid_address`,
  `fstat`, `lseek`, `ioctl`, `writev`, `prlimit64`, `futex`. Plus the
  ones that touch **files** — `open`, `openat`, `stat`, `access`: they
  answer "not found", which is the truth since there is no filesystem,
  and the program follows its error branch instead of stopping in front
  of the question; the console says so the first time. The others —
  starting a process, opening a connection — are named and reported as
  not simulated: behind namsdec there is neither network nor second
  process. `getrandom` returns **reproducible** bytes: exactly what a
  workshop where you step backwards needs, and exactly what a key must
  never have.
- AVX-512, FMA, the `in`/`out` ports and `cmpxchg16b` are not
  implemented.

### The window

namsdec draws its own title bar: the menus stand in for it, and the
three buttons are **at the head**, on the left — close, minimise,
maximise. Closing therefore occupies the corner of the screen, the angle
a mouse thrown without aiming reaches, and the other two stay behind it.
You move the window by grabbing the bar, maximise it with a double
click, resize it by its edges. Preferences ▸ General hands the desktop
decoration back to whoever prefers it — the setting applies without
restarting.

### Leaving the workshop

File ▸ **Export the binary** writes the program exactly as `ld` produced
it — an executable ELF, the one the command line would have given: it
runs in a terminal, reads with `objdump`, compares with what namsdec
says about it. The Diagnostics tab shows the **two commands actually
run**, ready to copy:

```sh
nasm -f elf64 -o program.o program.asm
ld -Ttext 0x400000 -o program program.o
```

A workshop that lets nothing out teaches you to use itself, not the
assembler.

### Analysis without the window

`namsdec-cli` writes out what the analysis panels show, without opening
an interface. Same substance, same path: the analysis was moved out of
the interface, and nothing is painted there.

```sh
namsdec-cli program                   # the report, on screen
namsdec-cli program -o report.md      # the report, to a file
namsdec-cli program --rubriques       # what this file has material for
namsdec-cli program -s protections    # a single section
```

**And what it makes possible, which is its reason to exist: comparing
two versions of a binary.** A protection that disappears, a string that
appears, a function that grows — a byte `diff` would say "the file
changed at offset 0x15604", which teaches nothing.

```sh
namsdec-cli program.v2 --contre program.v1
```

```diff
# What changed — program.v1 → program.v2

2 sections changed, 17 say the same thing.

## Binary hardening — changed

- - √ **NX** non-executable stack — Code dropped on the stack will not run.
+ - × **NX** executable stack — The stack is executable: bytes written there
+     can be run as they are.
+ - × **W+X segment** 0x0 — A region is both writable and executable.
- | GNU_STACK | rw- | 0x0 | 0 B |
+ | GNU_STACK | rwx | 0x0 | 0 B |
```

Above, **a single bit changed in the file** — the one that makes the
stack executable. A byte `diff` would have said "one byte changed"; the
analysis draws three sentences out of it, and the signal that goes with
them.

**The count of what did not change is the useful half of the report**:
"fifteen sections say the same thing" is what lets you believe the four
that moved. A section that appears or disappears entirely is said to
have done so, without reprinting its three hundred lines.

In CI, `-s protections` asks the only question that matters — the
fingerprints change on every build — and **the exit code carries the
answer**: `4` when the two analyses differ, as `diff` returns `1`.
Nothing to re-read to find out.

```sh
namsdec-cli program.v2 -c program.v1 -s protections || echo "a protection moved"
```

The report is **reproducible**: two calls on the same file return the
same text, because the command starts again from the bytes. That is
also what it does not do — it carries nothing from a session (no name
given to a function, no detour computed), and it launches no external
tool: the `objdump`, `radare2`, Ghidra and RetDec sections stay silent,
so that a report never speaks of the machine that produced it.

Its exit codes are made for a CI pipeline: `0` the report was written
(and, when comparing, nothing changed), `1` the command line is wrong,
`2` the file cannot be read, `3` it reads but there is nothing to
analyse — an empty or truncated artefact, which would otherwise pass for
analysed — `4` the two compared analyses differ. `namsdec-cli --aide`
says all the rest.

### Updates

*About ▸ Check for updates* asks GitHub for the published releases and
compares with yours — nothing is sent, and nothing leaves until you
click. **Which releases you are offered** is set in Preferences ▸
General: final releases are offered either way, and the box *adds* the
betas on top. By default it follows the version you are running.

**namsdec can install itself, once you agree.** When a newer release
exists, *Install this release…* opens a question naming the file that
would be replaced and what is about to happen; only the second button
starts anything. The binary is then taken from the GitHub release,
**its SHA-256 is checked before anything moves**, the current one is
kept beside it as `.ancien`, and the new one takes its place in a
single rename. It starts at the next launch: this session keeps running
on the old one. None of this happens at startup, and never unasked.

If the program's folder is not writable — a distribution package, a
binary installed by root — namsdec offers nothing and says so: updating
is then the package manager's job. From source, it is done by hand as
before:

```sh
git pull && ./install.sh
```

You can run that with namsdec open: the window stays there during the
compilation, and reopens by itself on the new version at the end.

A workshop that rewrites itself *while you are working inside it* is not
a workshop — which is why the installation asks first, and why nothing
is ever checked or fetched on startup.

On the repository side, pushing a `v…` tag compiles, checks (`fmt`,
`clippy`, every test) and publishes the binary with its SHA-256 sum; a
tag carrying a dash is published as a pre-release, as SemVer says and as
the program reads it.

## Changelog

What changes from one version to the next is in
[CHANGELOG.md](CHANGELOG.md).

## Licence

Written by **Frédéric Zawalski**.

MIT. See [LICENSE](LICENSE).
