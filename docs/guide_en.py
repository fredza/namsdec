"""Le guide de l'utilisateur, en anglais.

CE N'EST PAS UNE TRADUCTION, et c'est voulu. Le guide français fait
quarante mille mots ; en rendre chaque paragraphe donnerait deux textes
de cette taille à tenir à jour, et le second vieillirait le premier.
Celui-ci est écrit POUR SON LECTEUR : les mêmes chapitres, la même
matière, resserrés à ce dont on a besoin pour installer, prendre en
main, et lire un binaire qu'on n'a pas écrit.

Les NOMBRES viennent du dépôt, comme en français : ils sont passés en
paramètres et jamais écrits ici. Un guide qui annonce cent trente-huit
exercices le jour où il y en a cent quarante est pire qu'un guide muet.

Le cadre du PDF — couverture, sommaire, pied — vit dans `guide.py`,
sous `CADRE["en"]`. Ici, seulement le corps.

Pour l'écrire :  python3 docs/guide.py --lang en
"""


def contenu(v, total_exos, avec_but, paliers, nb_fiches, nb_tests, nb_fonctions,
            total_atelier, atelier_paliers):
    """Le guide anglais, chapitre par chapitre.

    Mêmes genres de blocs que le français : 'h1' un chapitre, 'h2' une
    section, 'p' un paragraphe, 'code' un bloc de code, 'liste' une
    énumération, 'table' un tableau, 'note' un encadré.

    Découpé en morceaux parce qu'un seul littéral de quarante pages ne
    se relit pas : chaque fonction tient un chapitre, et son nom dit
    lequel.
    """
    return (
        _ouverture(nb_fiches, nb_tests, total_exos, avec_but)
        + _interface()
        + _panneaux(nb_fonctions)
        + _syntaxes()
        + _exercices(total_exos, avec_but, paliers, total_atelier, atelier_paliers)
        + _encodage()
        + _retro()
        + _depannage()
        + _ligne_de_commande()
        + _reglages()
    )


def _ouverture(nb_fiches, nb_tests, total_exos, avec_but):
    """Ce que namsdec fait, l'installation, les premiers pas."""
    return [
        ("h1", "What namsdec does"),
        ("p",
         "namsdec is a workbench for <b>learning x86-64 assembly</b> on Linux, and for "
         "<b>reading binaries you did not write</b>. It assembles your program with the "
         "real <font face='Courier'>nasm</font>, links it with the real "
         "<font face='Courier'>ld</font>, then runs it one instruction at a time in a "
         "simulated processor whose every part it shows: the registers, the flags, the "
         "stack, the heap, memory and its permissions."),
        ("p",
         "The difference from a debugger fits in one sentence: a debugger shows you "
         "<i>what is happening</i>, namsdec explains <i>why</i>. The address an operand "
         "points at is worked out before execution, a conditional jump announces whether "
         "it will be taken and by which rule, a system call is written out the way you "
         "would write it in C, and a memory fault says what you should have done "
         "instead."),
        ("note",
         "This release is a <b>pre-release</b>. It is usable and tested — %d automated "
         "tests accompany it — but its interface and its file formats may still change "
         "from one version to the next." % nb_tests),

        ("h2", "Who it is for"),
        ("liste", [
         "People learning assembly who want to <b>see</b> what each instruction does.",
         "People who know a high-level language and want to understand what it becomes.",
         "People starting in reverse engineering who want a tool that explains rather "
         "than one that merely displays.",
         "Teachers: everything here is meant to be shown, copied out and handed on. "
         "MIT licence.",
        ]),

        ("h2", "What is in the box"),
        ("liste", [
         "A <b>processor</b> simulated instruction by instruction: integers, SSE/AVX, "
         "the x87 stack, system calls, a heap, and memory protected by the real "
         "permissions of the sections.",
         "An <b>editor</b> with syntax colouring, completion that only offers what is "
         "documented, and — at advanced level — vim's three modes.",
         "A <b>manual</b> of %d instructions, directives and concepts, under <b>F1</b> "
         "or on hover, exportable as Markdown." % nb_fiches,
         "A <b>path of %d exercises</b> in four levels, each one carrying its own theory "
         "card, %d of them with a checkable goal." % (total_exos, avec_but),
         "A <b>reverse-engineering</b> window: signals, hardening, cross-references, "
         "strings, behaviour families, packers, ROP gadgets, slack space, a binary "
         "editor and a detour tool.",
         "A <b>command line</b>, <font face='Courier'>namsdec-cli</font>, which writes "
         "the same analysis without opening a window, and compares two versions of a "
         "binary.",
        ]),

        ("h1", "Installing"),
        ("p",
         "You need <b>Rust</b> (<font face='Courier'>cargo</font>), <b>nasm</b> and "
         "<b>ld</b> (binutils). namsdec calls the last two for real: what it assembles "
         "is what your command line would have assembled."),
        ("code",
         "# Fedora\n"
         "sudo dnf install nasm binutils\n"
         "# Debian, Ubuntu\n"
         "sudo apt install nasm binutils"),
        ("code",
         "git clone https://github.com/fredza/namsdec.git\n"
         "cd namsdec\n"
         "./install.sh"),
        ("p",
         "<font face='Courier'>install.sh</font> compiles, puts the binary in "
         "<font face='Courier'>~/.local/bin</font>, and the icon and launcher where the "
         "desktop looks for them. <b>Nothing is written outside your home directory, and "
         "nothing asks for administrator rights.</b>"),
        ("note",
         "<b>Running the script again is an update, not a reinstall.</b> Your settings, "
         "the exercise packs you added and your breakpoints stay where they are, and the "
         "script says so before it starts. If namsdec is open it stays open for the whole "
         "job; only at the very end does the script close it cleanly and reopen it on the "
         "new version. A window that refuses to go — a question on screen, unsaved work — "
         "is never forced."),

        ("h2", "The options worth knowing"),
        ("table", [
         ["option", "what it does"],
         ["--dry-run, -n", "say what would be done, write nothing"],
         ["--debug", "install the unoptimised build, ready in seconds"],
         ["--prefix DIR", "where icons, launcher and exercises go"],
         ["--bin-dir DIR", "where the binary goes"],
         ["--no-desktop", "no launcher, no icons"],
         ["--no-exercises", "do not install the bundled pack"],
         ["--no-restart", "do not close and reopen namsdec if it is running"],
         ["--no-build", "do not compile: install the binary already built"],
        ]),
        ("p",
         "<font face='Courier'>./uninstall.sh</font> undoes exactly that. "
         "<font face='Courier'>--purge</font> also removes your settings and exercise "
         "packs — without it they are kept, because people usually uninstall in order to "
         "reinstall. To run without installing at all: "
         "<font face='Courier'>cargo run -p namsdec-gui</font>."),
        ("p",
         "<b>A lighter binary:</b> <font face='Courier'>cargo build --release "
         "--no-default-features</font> drops the update check and the TLS stack with it, "
         "ten per cent of the size. That is what a distribution packager wants: updates "
         "are their job, not the application's. The menu then does not show the entry, "
         "rather than offering a button that can do nothing."),

        ("h1", "First steps"),
        ("p",
         "namsdec opens on the <b>Workbench</b>: the beginner layout. Four fixed bands, "
         "nothing to open and nothing to configure — the program in the middle, what the "
         "machine does on the right, the result at the bottom. The <b>Advanced mode</b> "
         "button, top right, gives you the full interface: menus, panels, rails, status "
         "bar. The choice is kept."),
        ("table", [
         ["step", "how"],
         ["Assemble", "Ctrl+Shift+B — nasm, then ld, for real"],
         ["One instruction", "Space s i (stepi)"],
         ["Over a call", "Space n i"],
         ["Run", "Space p r"],
         ["One step backwards", "Space p b"],
         ["To the cursor", "Ctrl+F10"],
         ["The card for a word", "F1, or hover it"],
         ["Every command", "Ctrl+Shift+P"],
        ]),
        ("p",
         "The first two keep gdb's letters because they are the most hit keys in the "
         "whole program. The rest of the sequences are sorted by family: "
         "<font face='Courier'>p</font> for what runs the <b>program</b>, "
         "<font face='Courier'>/</font> to search, <font face='Courier'>m</font> to "
         "<b>mark</b>, <font face='Courier'>v</font> for the <b>views</b>, "
         "<font face='Courier'>u</font> to run <b>until</b> something. Every shortcut can "
         "be changed in Preferences › Shortcuts."),
        ("note",
         "<b>At beginner level the editor has no modes.</b> You click, you type, it is "
         "written. At advanced level it has three, like vim: NORMAL (the "
         "<font face='Courier'>hjkl w b e gg G</font> motions and the "
         "<font face='Courier'>5j</font> counter), INSERT (<font face='Courier'>i</font>, "
         "Escape to leave) and VISUAL (<font face='Courier'>v</font>, then "
         "<font face='Courier'>y</font> to copy and <font face='Courier'>d</font> to "
         "cut). The status bar says which one you are in."),

        ("h2", "Predict before you step"),
        ("p",
         "Telling you what the next instruction will do, before you press the key, hands "
         "you the answer before the question: you never get the chance to be wrong — and "
         "being wrong is the only way to find out you had not understood. In beginner "
         "mode the address and the instruction stay on screen, the answer waits for the "
         "step, and <i>Last step</i> reveals it beside what actually changed."),
        ("code",
         "Next instruction\n"
         "  0x400005  mov ebx, 2\n"
         "  What is this going to change?   [Show]\n"
         "\n"
         "Last step  0x400000  mov eax, 1\n"
         "  What it said: EAX = 1\n"
         "  RAX 0x0 -> 0x1"),
        ("p",
         "<i>Show</i> gives the answer straight away, one click and no confession. The "
         "setting lives in Preferences › Modules, under beginner mode: it only makes "
         "sense there, since an exercise forced on somebody who uses the tool daily would "
         "be a tax."),

        ("h2", "The console is not there at the start"),
        ("p",
         "Before you have assembled anything the console would have one thing to say: "
         "that it is empty. It appears by itself as soon as it has something to show — an "
         "error from nasm, or the first byte your program writes. Closed mid-run it stays "
         "closed; View › Console opens it on request, and the setting is kept."),
    ]


def _panneaux(nb_fonctions):
    """Les panneaux, le pas à pas, la mémoire et le tas."""
    return [
        ("h1", "The panels, one by one"),
        ("p",
         "The right-hand panel is what the machine holds. Its tabs — <b>Processor</b>, "
         "<b>Stack</b>, <b>Memory</b>, <b>Heap</b>, <b>Bytes</b> — all answer the same "
         "question from different angles: where is the value I am looking for, and what "
         "put it there?"),

        ("h2", "Processor"),
        ("p",
         "The sixteen general registers, in hexadecimal, with the ones that changed at "
         "the last step picked out. Hovering a register says <b>which step wrote it "
         "last</b>, its rank and its line — \"written at step -12, line 7: add rax, "
         "rcx\". Finding that by hand means reading your program backwards, and getting "
         "it wrong every other time as soon as there is a loop."),
        ("p",
         "Underneath, the seven flags: OF, DF, SF, ZF, AF, PF, CF. <b>They are read, "
         "never clicked</b> — they are the result of the last computation, not a switch. "
         "A register value, on the other hand, <i>is</i> set by hand: click it, type, "
         "press Enter. \"What if RCX were 1?\" is answered in a second, without "
         "rewriting the program."),
        ("note",
         "<b>Extended registers</b> — SSE, AVX, x87 — live behind their own link, because "
         "sixteen 256-bit vectors on screen push everything else off it."),

        ("h2", "Stack"),
        ("p",
         "One eight-byte word per line, RSP and RBP marked, and any value pointing into "
         "the code named for what it is: a return address, with its label. What sits "
         "above RSP is greyed out — those bytes are still there, but the next "
         "<font face='Courier'>push</font> overwrites them. A <b>Return addresses</b> "
         "reading lists only those, which is what you want in front of an overflow."),

        ("h2", "Memory"),
        ("p",
         "A hexadecimal dump with its address, its bytes and its text, and the "
         "<b>permissions of the real section</b> applied: writing into "
         "<font face='Courier'>.rodata</font> fails here as it would on the machine, and "
         "says why. Three things live in this tab and are worth knowing about:"),
        ("liste", [
         "<b>Memory map</b> — what is mapped where, with the permissions. The "
         "equivalent of <font face='Courier'>vmmap</font>.",
         "<b>Search memory</b> — a string or a sequence of bytes, with every hit listed.",
         "<b>Write here</b> — change a byte in the simulated memory, which "
         "<i>step back</i> undoes like everything else.",
        ]),

        ("h2", "Heap"),
        ("p",
         "<font face='Courier'>malloc</font> and its family take their memory from a "
         "simulated heap grown with <font face='Courier'>brk</font>, and this tab shows "
         "it: each block, its size, whether it is free. A leak is visible, and so is a "
         "double free — the two things a printed pointer never tells you."),

        ("h2", "Bytes"),
        ("p",
         "The instruction under the cursor, byte by byte, with each part named: the "
         "prefix, the opcode, the ModRM byte, the displacement, the immediate. This is "
         "where you see that <font face='Courier'>mov rax, 0</font> is five bytes and "
         "not seven, because the assembler goes through "
         "<font face='Courier'>eax</font>."),

        ("h1", "Stepping, and going back"),
        ("p",
         "Every step is recorded. <b>Step back</b> is not a re-run: it restores the "
         "state before the instruction — registers, flags, memory, heap and the console "
         "with them. Ten minutes of execution are not lost because you went one "
         "instruction too far."),
        ("p",
         "The <b>Timeline</b> view turns that record into a list you can click: pick a "
         "past step and the whole machine returns to it. The <b>Profile</b> tab of the "
         "console ranks the lines by the number of passes, with the share of the total "
         "and a click that takes you there; the gutter shows the same thing as a heat "
         "band, green for a line walked once, red for the hottest."),

        ("h2", "Breakpoints"),
        ("p",
         "A click in the gutter sets one. Each can carry a <b>condition</b> — "
         "<font face='Courier'>rcx == 0</font>, <font face='Courier'>rax > 0x100</font>, "
         "<font face='Courier'>(rax &amp; 0xff) == 'A'</font>, "
         "<font face='Courier'>[rsp+8] != 0</font> — written in the calculator's own "
         "language, memory included. Left empty, it stops at every pass. The "
         "<b>Breakpoints</b> view lists them with the registers each one saw go by."),
        ("p",
         "You can also watch a <b>register</b> or an <b>address</b>: execution stops when "
         "the value changes, and says what it was and what it became. And a "
         "<b>calling-convention check</b> reports what a subroutine borrowed without "
         "giving back."),

        ("h2", "The calculator"),
        ("p",
         "%d functions, the same language as the breakpoint conditions: arithmetic, bit "
         "operations, registers by name, memory by address. It is the place to answer "
         "\"what would that give?\" without touching the program — and the same engine "
         "reads the conditions, so what works in one works in the other." % nb_fonctions),

        ("h2", "The flow graph"),
        ("p",
         "<font face='Courier'>Space g</font> opens it. Each box is a block you can only "
         "enter from the top and leave from the bottom; arrows show the taken jump "
         "(green), the untaken one (red) and the call (blue), and a loop shows up by "
         "going back up. The block the program is in is brought forward, a click takes "
         "you there in the editor, and Ctrl+wheel zooms out to see the overall shape."),
        ("p",
         "A field there takes an <b>address</b> — the one where the program accepts, say "
         "— and gives the <b>route to reach it</b>, with the conditions to satisfy in "
         "order: this <font face='Courier'>jne</font> must not be taken, that one must. "
         "It is the shortest route, and namsdec says when others exist rather than "
         "letting you believe in a proof."),
    ]


def _exercices(total_exos, avec_but, paliers, total_atelier, atelier_paliers):
    """Le parcours : ce qu'il enseigne, et comment il se vérifie."""
    rangs = [("Discovery", "Découverte"), ("Practice", "Pratique"),
             ("Structures", "Structures"), ("Mastery", "Maîtrise")]
    table = [["level", "steps", "what it covers"]]
    couverture = {
        "Découverte": "the frame of a program, bits and bytes, the registers and their "
                      "slices, mov, add and sub, the jump before the loop, flags, "
                      "two's complement",
        "Pratique": "shifts, multiplication and division, memory, arrays and matrices, "
                    "address computation, branchless clamping, and the first steps of "
                    "reverse engineering",
        "Structures": "strings, records, jump tables, the stack, calls and the calling "
                      "convention, recursion, sorting, linked lists",
        "Maîtrise": "output and input, the heap, floating point, SIMD, overflow, "
                    "hardening, XOR encryption, a canary placed by hand",
    }
    for anglais, français in rangs:
        n = paliers.get(français, 0)
        table.append([anglais, str(n), couverture[français]])
    return [
        ("h1", "The %d exercises" % total_exos),
        ("p",
         "A path of %d exercises ships with the program, in four levels of rising "
         "difficulty. It is <b>not in the binary</b>: it is a data file, and yours is "
         "added the same way." % total_exos),
        ("table", table),

        ("h2", "Each one carries its theory"),
        ("p",
         "Not the brief, not the hint. The brief says what is asked; the hint says where "
         "to look when you are stuck; the <b>theory</b> says the mechanism the exercise "
         "rests on — what a loop is when the processor has none, why writing "
         "<font face='Courier'>eax</font> wipes the top of <font face='Courier'>rax</font>, "
         "what a calling convention is not obliged to guarantee. You read it before "
         "trying, and it will serve ten other steps."),
        ("p",
         "It is written for a beginner: one idea per paragraph, short sentences, "
         "<b>an example with real numbers</b>, and the trap at the end. It quotes between "
         "backticks the words that have a documentation card, and the interface turns "
         "those into clickable cross-references. The <b>Theory</b> button sits in the "
         "gesture row, next to <i>A hint</i> and <i>The solution</i> — the order in which "
         "you come to them — and it opens a window of its own."),
        ("note",
         "Nothing here is held by hand. A test checks that every theory, in both "
         "languages, quotes at least one card that really exists, does not merely repeat "
         "its brief, carries a worked example with numbers, and fits within the reading "
         "bounds of its window."),

        ("h2", "Goals, budgets, and reading exercises"),
        ("p",
         "%d of the %d carry a <b>checkable goal</b> — <font face='Courier'>rax == "
         "0xf</font> — and the processor banner says \"reached\" or \"not yet\" instead "
         "of leaving you to verify by eye. What has been passed once stays passed: a "
         "badge in the list, a count at the head of the pack, and both survive the next "
         "launch." % (avec_but, total_exos)),
        ("p",
         "Each measured exercise also carries a <b>budget</b>: the number of instructions "
         "the shipped solution executes. Matching it is normal; going below is the real "
         "exercise — it is while looking for how that you learn "
         "<font face='Courier'>lea</font>, shifts, or that a five-round loop can be "
         "replaced by a formula. The best score is kept per exercise, and a ring marks "
         "those under budget."),
        ("p",
         "Some exercises ask you to <b>read</b> rather than to write: the program is "
         "given complete, a question is asked — \"what is RAX at the halt?\" — and you "
         "answer before running. The verdict does not hand you the right answer: you get "
         "it by running, that is, by doing what the exercise teaches. "
         "<font face='Courier'>32</font>, <font face='Courier'>0x20</font>, "
         "<font face='Courier'>20h</font> and <font face='Courier'>0b100000</font> are "
         "the same answer, and whoever gives it in binary understood the same thing."),
        ("note",
         "<b>The answer written in the pack is the one the machine gives.</b> A test "
         "assembles each of those programs, runs it and checks: an answer written by "
         "hand in a JSON file is a claim; put to the emulator it is a fact."),

        ("h2", "Adding your own pack"),
        ("p",
         "A JSON file in <font face='Courier'>~/.local/share/namsdec/exercises/</font>. "
         "Each exercise carries a title, a brief, a starting program, a hint and a "
         "solution; <font face='Courier'>goal</font> adds a checkable condition, and "
         "<font face='Courier'>question</font> with <font face='Courier'>answer</font> "
         "makes a reading exercise. A second bundled pack, the Workshop, adds %d more "
         "steps and is written exactly that way — it is the worked example of the "
         "format." % total_atelier),
    ]


def _retro():
    """Lire un binaire qu'on n'a pas écrit."""
    return [
        ("h1", "Reading a binary you did not write"),
        ("p",
         "File › Open accepts an ELF64. namsdec loads it the way the kernel would — "
         "segments mapped into pages, relocations applied, thread-local memory laid "
         "down, the process stack written with <font face='Courier'>argc</font>, "
         "<font face='Courier'>argv</font>, the environment and the auxiliary vector — "
         "and the Analysis window opens on what it found."),

        ("h2", "Signals: where to start"),
        ("p",
         "A fooled listing, a segment both writable <i>and</i> executable, rising "
         "entropy, a packer recognised by name, anti-debugging, missing protections, "
         "flags hiding in strings. <b>Nothing new is computed there</b> — it is what the "
         "other panels already found, gathered to say where to start. Each line gives a "
         "fact (an address, a name, a measurement), what that fact changes, and the panel "
         "that explains it."),
        ("note",
         "Nothing is added up into a score. A score would look like a verdict, and it is "
         "the reader who concludes."),

        ("h2", "Hardening"),
        ("p",
         "PIE, NX, RELRO, stack canary, FORTIFY, IBT and shadow stack, W+X segments, "
         "symbols, libraries, RPATH, and the functions it borrows — each relocation entry "
         "gets its name back, with the number of calls leading to it. For each, what its "
         "state <i>actually changes</i>, not merely whether it is on. The whole report "
         "copies out as Markdown."),

        ("h2", "When the listing lies"),
        ("p",
         "A disassembler reads <i>linearly</i>; the processor goes <b>where the jumps "
         "send it</b>. One trap byte behind a <font face='Courier'>jmp</font> that steps "
         "over it, and the whole line-by-line reading goes askew. The <b>Code shapes</b> "
         "panel gives both readings side by side and the byte that separates them."),
        ("p",
         "What you learn on the way is worth more than the finding itself: where "
         "instructions begin is not a property of the bytes, it is a property of where "
         "you start reading."),

        ("h2", "What the binary can do"),
        ("p",
         "Imports and system calls are listed everywhere; here they are also <i>sorted</i> "
         "into six families — network, execution, anti-debugging, memory, cryptography, "
         "files. The question is not \"which functions\" but \"what does this binary "
         "do\", and three lit families are already a portrait. A "
         "<font face='Courier'>socket</font> is a <font face='Courier'>socket</font> "
         "whether it arrives through the relocation table or as number 41 in RAX — and a "
         "program trying to go unnoticed uses the second."),

        ("h2", "Where to start reading"),
        ("p",
         "The <b>Functions</b> module recovers <font face='Courier'>main</font> in a "
         "stripped binary — the startup code hands its address to "
         "<font face='Courier'>__libc_start_main</font>, in RDI — then cuts the code at "
         "call targets, with the number of callers for each. <b>Cross-references</b> "
         "record, for each address, who calls it, who jumps to it and who names it: an "
         "address a <font face='Courier'>call</font> aims at is a function even stripped "
         "of its name, and the one called from everywhere is the one to begin with."),
        ("p",
         "The <b>Shapes</b> module picks out constants that are really text, length "
         "checks, comparisons disguised as XOR — <font face='Courier'>a XOR b</font> is "
         "zero only if the two are equal, so the constant <i>is</i> the expected value — "
         "string comparisons and jump tables. Families are sorted from the rarest to the "
         "most common: in a binary of sixty thousand instructions, seventeen hundred zero "
         "tests are punctuation, and four XOR comparisons are a lead."),

        ("h2", "The binary editor, and the detour"),
        ("p",
         "The <b>binary editor</b> shows the file's position <i>and</i> the address in "
         "memory, side by side. You pick a byte, you put another one there, and every "
         "write is undone from Tools › Patches. Nothing touches the disk unless you save, "
         "to a file you name."),
        ("p",
         "The <b>detour</b> adds code to an executable without shifting it: one byte more "
         "would falsify every address that follows, so nothing is pushed — a jump is "
         "placed into a hole the binary already contains, the new code is written there, "
         "the overwritten instructions are replayed, and it comes back. The file keeps "
         "its exact size. The panel refuses what it cannot relocate, and says so site by "
         "site."),

        ("h2", "External tools, if you have them"),
        ("p",
         "The <b>Decompilation</b> module (Preferences › Modules, off by default) runs "
         "the tool of your choice: <font face='Courier'>objdump</font>, "
         "<font face='Courier'>ndisasm</font>, radare2's pseudo-code, Rizin + rz-ghidra, "
         "RetDec. Each card answers two independent questions — is it <b>installed on "
         "this machine</b>, and is it <b>enabled in namsdec</b>?"),
        ("note",
         "Three locks, because this means running a program on your machine: the module "
         "is enabled by hand, the window opens on request, and <b>the command is spelled "
         "out in full before it is run</b>. No step calls <font face='Courier'>sudo</font>, "
         "no step goes through a shell, and nothing is placed outside "
         "<font face='Courier'>~/.local/share/namsdec/tools/</font>."),
    ]


def _ligne_de_commande():
    """`namsdec-cli` : la même analyse, sans fenêtre."""
    return [
        ("h1", "The command line"),
        ("p",
         "<font face='Courier'>namsdec-cli</font> writes out what the analysis panels "
         "show, without opening an interface. Same substance, same path: the analysis "
         "was moved out of the interface, and nothing is painted there."),
        ("code",
         "namsdec-cli program                   # the report, on screen\n"
         "namsdec-cli program -o report.md      # the report, to a file\n"
         "namsdec-cli program --rubriques       # what this file has material for\n"
         "namsdec-cli program -s protections    # a single section"),

        ("h2", "Comparing two versions"),
        ("p",
         "This is its reason to exist. A protection that disappears, a string that "
         "appears, a function that grows — a byte <font face='Courier'>diff</font> would "
         "say \"the file changed at offset 0x15604\", which teaches nothing."),
        ("code", "namsdec-cli program.v2 --contre program.v1"),
        ("p",
         "Where a single bit changed in the file — the one that makes the stack "
         "executable — the analysis draws three sentences out of it and the signal that "
         "goes with them. <b>The count of what did not change is the useful half of the "
         "report</b>: \"fifteen sections say the same thing\" is what lets you believe "
         "the four that moved."),
        ("p",
         "In CI, <font face='Courier'>-s protections</font> asks the only question that "
         "matters — fingerprints change on every build — and <b>the exit code carries "
         "the answer</b>."),
        ("code",
         "namsdec-cli program.v2 -c program.v1 -s protections \\\n"
         "  || echo \"a protection moved\""),
        ("table", [
         ["code", "what it means"],
         ["0", "the report was written; comparing, nothing changed"],
         ["1", "the command line is wrong"],
         ["2", "the file cannot be read"],
         ["3", "it reads, but there is nothing to analyse — empty or truncated"],
         ["4", "the two compared analyses differ"],
        ]),
        ("note",
         "The report is <b>reproducible</b>: two calls on the same file return the same "
         "text, because the command starts again from the bytes. It carries nothing from "
         "a session — no name given to a function, no detour computed — and it launches "
         "no external tool, so that a report never speaks of the machine that produced "
         "it."),

        ("h1", "Leaving the workshop"),
        ("p",
         "File › <b>Export the binary</b> writes the program exactly as "
         "<font face='Courier'>ld</font> produced it: an executable ELF, the one the "
         "command line would have given. It runs in a terminal, reads with "
         "<font face='Courier'>objdump</font>, and compares with what namsdec says about "
         "it. The Diagnostics tab shows the two commands actually run, ready to copy."),
        ("code",
         "nasm -f elf64 -o program.o program.asm\n"
         "ld -Ttext 0x400000 -o program program.o"),
        ("p",
         "A workshop that lets nothing out teaches you to use itself, not the "
         "assembler. The manual exports the same way: Help › export writes every "
         "documentation card as Markdown, because what you learn here must be readable "
         "elsewhere."),
    ]


def queue():
    """La fin du guide : les fichiers, les limites, les mises à jour.

    Le pendant anglais de ce que `guide.py` ajoute après le chapitre des
    raccourcis — qui, lui, est dressé depuis le code dans les deux
    langues.
    """
    return [
        ("h1", "Where the files live"),
        ("table", [
         ["path", "what it holds"],
         ["~/.local/bin/namsdec", "the program"],
         ["~/.config/namsdec/settings.json", "every setting of yours"],
         ["~/.config/namsdec/keymap.json", "the active shortcut set"],
         ["~/.config/namsdec/keymaps/", "the shortcut sets you saved"],
         ["~/.local/share/namsdec/exercises/", "the path and your own packs"],
         ["~/.local/share/namsdec/themes/", "the themes you installed"],
         ["~/.local/share/namsdec/modules/", "the external tools you declared"],
         ["~/.local/share/namsdec/tools/", "the tools namsdec fetched for you"],
        ]),

        ("h1", "What namsdec does not do"),
        ("liste", [
         "No 32-bit mode and no <font face='Courier'>int 0x80</font>: only x86-64 is "
         "simulated.",
         "<b>No dynamic linker.</b> namsdec loads no library, and the code of "
         "<font face='Courier'>printf</font> is nowhere in the simulated memory. Instead "
         "it <b>provides one hundred and seventy functions</b> written in Rust — "
         "printing, reading, cutting and comparing strings, classifying characters, "
         "asking for memory. A binary that borrows anything else stops by <b>naming</b> "
         "the function; a statically linked binary borrows nothing and runs in full.",
         "<b>No filesystem.</b> <font face='Courier'>fopen</font>, and the "
         "<font face='Courier'>open</font>, <font face='Courier'>openat</font>, "
         "<font face='Courier'>stat</font> and <font face='Courier'>access</font> system "
         "calls, all answer \"not found\" — which is the truth, since there is no file — "
         "and let the program follow its error branch, often the half you came to see. "
         "The console says so the first time.",
         "<b>No second process and no network.</b> <font face='Courier'>fork</font>, "
         "<font face='Courier'>execve</font>, <font face='Courier'>socket</font>, "
         "<font face='Courier'>connect</font> are named and reported as not simulated. A "
         "program containing one would stop there without warning, so this is said at "
         "assembly time, with the line.",
         "<font face='Courier'>getrandom</font> returns <b>reproducible</b> bytes: "
         "exactly what a workshop where you step backwards needs, and exactly what a key "
         "must never have.",
         "AVX-512, FMA, the <font face='Courier'>in</font>/<font face='Courier'>out</font> "
         "ports and <font face='Courier'>cmpxchg16b</font> are not implemented.",
        ]),
        ("note",
         "Everything provided obeys the same rules as the rest: the simulated memory "
         "permissions apply — a <font face='Courier'>strcpy</font> into "
         "<font face='Courier'>.rodata</font> fails, as it would on the real machine — "
         "each call counts as <b>one step</b>, and what it writes is undone by "
         "<i>step back</i>."),

        ("h1", "Reporting a problem"),
        ("p",
         "Help › <b>Report a problem</b> opens a window that writes the report for you: "
         "what you were doing, what you expected, what happened, and the state of the "
         "machine — version, build date, assembler found, the program loaded. You read "
         "it before anything leaves, and nothing leaves on its own: the window copies the "
         "text or opens the issue page in your browser, at your click."),
        ("p",
         "A good report says three things: <b>what you did</b>, <b>what you expected</b>, "
         "and <b>what you got instead</b>. The third alone is a mystery; the three "
         "together are a defect."),

        ("h1", "Updates, licence and contact"),
        ("p",
         "<i>About › Check for updates</i> asks GitHub for the published releases and "
         "compares with yours. Nothing is sent, and nothing leaves until you click — "
         "there is no check at startup. <b>Which releases you are offered</b> is set in "
         "Preferences › General: final releases are offered either way, and the box "
         "<i>adds</i> the betas on top. By default it follows the version you are "
         "running."),
        ("p",
         "<b>Since 27 November 2026 namsdec can install itself — once you agree.</b> "
         "When a newer release exists, <i>Install this release…</i> opens a question "
         "naming <i>which file</i> would be replaced and <i>what is about to happen</i>; "
         "only the second button starts anything. The first click downloads nothing."),
        ("p",
         "What happens then, in order: the binary is taken from the GitHub release; "
         "<b>its SHA-256 is checked as it arrives</b>, and a file that does not match is "
         "deleted without anything having moved; it is checked to be an executable and "
         "not an error page that arrived without an error; the current one is kept "
         "beside it as <font face='Courier'>.ancien</font>; and the new one takes its "
         "place by a <i>rename</i>, which is atomic — at no instant does the program's "
         "place hold a half-written file. A progress bar says where the download is, and "
         "<i>Stop</i> cuts it."),
        ("note",
         "The new version only starts at the <b>next launch</b>: this session keeps "
         "running on the old one, and that is deliberate — replacing the code under a "
         "working program is not an update, it is an accident. If the program's folder "
         "is not writable — a distribution package, a binary installed by root — namsdec "
         "offers nothing and says so: updating is then the package manager's job."),
        ("note",
         "<b>What the checksum proves, and what it does not.</b> It comes from the same "
         "release as the binary: it shows the bytes arrived whole, not that they are "
         "honest. What holds provenance here is transport encryption and the "
         "repository's GitHub account. The day releases are signed, the signature will "
         "take that role."),
        ("code", "git pull && ./install.sh"),
        ("p",
         "You can run that with namsdec open: the window stays there during the "
         "compilation and reopens by itself on the new version at the end. A workshop "
         "that rewrites itself while you are working inside it is not a workshop."),
        ("p",
         "namsdec is written by <b>Frédéric Zawalski</b> and published under the "
         "<b>MIT</b> licence: what you understand here is meant to be passed on. The "
         "site is <b>fredza.github.io/namsdec</b>, the code and the issues are on GitHub, and the "
         "About box leads to both as well as to the Discord room."),
    ]


def _interface():
    """La fenêtre : ce qu'on y trouve, et comment on s'y déplace."""
    return [
        ("h1", "Finding your way around"),
        ("p",
         "namsdec draws its own title bar: the menus stand in for it, and the three "
         "buttons are <b>at the head, on the left</b> — close, minimise, maximise. "
         "Closing therefore occupies the corner of the screen, the angle a mouse thrown "
         "without aiming reaches. You move the window by grabbing the bar, maximise it "
         "with a double click, resize it by its edges. Preferences › General hands the "
         "desktop decoration back to whoever prefers it, and the setting applies without "
         "restarting."),

        ("h2", "The icon rails"),
        ("p",
         "The explorer and the exercises on the left, the console at the bottom left, "
         "the processor panel on the right — <b>each icon on the side and at the height "
         "where its panel opens</b>. Hovered, it says what a click will do: \"Show the "
         "explorer\", \"Hide the explorer\", with the shortcut as you have set it."),
        ("p",
         "They command nothing the View menu does not command already. What they add is "
         "making visible what exists: a closed panel left no trace on the screen, and you "
         "do not close what you do not know how to reopen. Each panel also carries a dash "
         "at the end of its tab row, which puts it away."),

        ("h2", "Several files at once"),
        ("p",
         "A tab bar above the editor, the name shortened <b>in the middle</b> when it is "
         "long — so that <font face='Courier'>loop_i…ee.asm</font> and "
         "<font face='Courier'>loop_i…e_2.asm</font> stay two different files — and a dot "
         "on what is not saved. An open exercise takes its tab too, under its title."),
        ("p",
         "Each tab keeps <b>everything</b> you were doing in it: the text typed, the "
         "cursor, the breakpoints, and execution stopped at the step where you left it. "
         "Ctrl+Tab moves to the next, Ctrl+W closes, and the files come back on the next "
         "launch."),

        ("h2", "The views row"),
        ("p",
         "Source, Disassembly, Flow, Analysis, Timeline, Breakpoints, Extended registers, "
         "Patches. The row <b>fits on one line</b>: what does not fit goes behind a "
         "button that says how many — <font face='Courier'>+5</font> — and the view you "
         "are looking at always stays visible. Any view can be <b>detached</b> into a "
         "window of its own, to read it beside the source; closing that window brings it "
         "back into the row."),

        ("h2", "The command palette"),
        ("p",
         "Ctrl+Shift+P. Every command is in there, with its shortcut — and so are your "
         "program's <b>labels</b>: type a label name and you are on it. It is the fastest "
         "way to find a command whose shortcut you have forgotten, and the only one that "
         "does not require knowing which menu it lives in."),

        ("h2", "The home screen"),
        ("p",
         "Close the last file and the workshop shows its name, then what you start again "
         "from — new file, open, command palette, exercises — and the recent files, which "
         "a click reopens. Every shortcut shown there is read from the shortcut table, "
         "not copied into the page."),
        ("note",
         "It is <b>asked for</b>, by a gesture: closing. It is not deduced from an empty "
         "buffer, otherwise it would appear in the middle of typing — you select all, you "
         "delete to start again, and the editor would vanish under your fingers."),

        ("h2", "The count of problems"),
        ("p",
         "At the right edge of the views row: a red disc and the number of errors, an "
         "orange triangle and the number of warnings. A click opens the list on the right "
         "tab; the two chevrons lead to the previous and the next problem, wrapping "
         "around. <b>It only appears if there is something to count</b> — a counter "
         "permanently at zero is a place on the screen you learn to stop looking at."),

        ("h2", "Themes"),
        ("p",
         "Four themes ship with the program — VS Code Dark+, VS Code Light+, Tokyo Night "
         "and <b>Noël</b> — and a fifth is whatever you install: a "
         "JSON file in <font face='Courier'>~/.local/share/namsdec/themes/</font>, "
         "colours and nothing else — a theme does not execute. Preferences › General "
         "lists them, each with a thumbnail."),
        ("p",
         "<b>A theme's thumbnail is an editor in miniature</b>: the gutter, four lines "
         "of code in the colours of their tokens — label, mnemonic, register, number, "
         "comment, directive, string —, the line under the cursor, and the side panel "
         "with the accent, a settled value and one that has just changed. Status keeps "
         "its three dots: healthy, warning, broken. It used to be four coloured dots, "
         "none of which was a colour of the code you were about to read — and on "
         "Noël, two of those four were near-identical golds."),
        ("p",
         "<b>Noël</b> is the dark, understated one: three layers of near-black spruce for "
         "the backgrounds, snow for the ink, candle gold for the accent. Christmas red and "
         "green are also the colours of an error and of a success, so the mnemonic takes a "
         "holly red kept clear of the error red, and what has just changed — a register an "
         "instruction wrote to — gets a frost green of its own. It holds the same contrast "
         "floor as the other three, measured on every tab."),
        ("note",
         "The shipped themes hold a <b>contrast floor</b>: every active text stands out "
         "from its background by at least 3:1, the accessibility recommendation. That is "
         "not an intention — a test measures what is painted, composes each text with the "
         "background actually under it, and refuses to compile below the floor. A theme "
         "installed afterwards is not subject to it, and that is its author's business."),
    ]


def _syntaxes():
    """Trois façons de lire la même instruction, et comment en ajouter."""
    return [
        ("h1", "Three ways to read the same instruction"),
        ("table", [
         ["engine", "what it shows", "example"],
         ["Pseudo-code", "the instruction said in plain words", "RAX = RBX"],
         ["NASM", "the syntax you write in the editor", "mov rax, rbx"],
         ["System", "the one gdb, objdump and as speak", "mov %rbx, %rax"],
        ]),
        ("p",
         "The <b>System</b> syntax is a view, not a way of writing: nasm does not read "
         "it. It is there because everything you find outside namsdec — a manual page, a "
         "disassembly pasted into a forum, the output of "
         "<font face='Courier'>objdump</font> — is written in it, and translating in your "
         "head while learning is one thing too many."),
        ("p",
         "<b>In beginner mode the row offers only three</b>: pseudo-code, NASM, and "
         "<font face='Courier'>objdump</font> in Intel syntax — the one on every machine, "
         "and whose syntax is the one you have just learnt by writing nasm. Six ways of "
         "reading the same code in front of somebody who knows none of them is not a "
         "choice, it is one more question. An engine chosen at advanced level keeps its "
         "place if you go back to beginner level: you do not take away what somebody is "
         "in the middle of reading."),

        ("h2", "Adding a disassembler"),
        ("p",
         "One JSON file per tool, in "
         "<font face='Courier'>~/.local/share/namsdec/modules/</font>. The "
         "<font face='Courier'>kind</font> field says whether the card returns "
         "instructions (<font face='Courier'>disassembler</font>, the default) or "
         "pseudo-code (<font face='Courier'>decompiler</font>). Put "
         "<font face='Courier'>{out}</font> in the arguments if the tool writes its "
         "result to a file rather than to standard output."),
        ("note",
         "<b>If the tool does not read ELF</b>, two further markers let it off: namsdec "
         "hands it the executable bytes and their address instead. That is how DZF, the "
         "in-house disassembler, is wired — it has no ELF reader and does not need one."),
        ("p",
         "The work happens alongside, so the interface does not freeze, and a tool that "
         "has not come back after two minutes is stopped rather than waited on forever. "
         "Progress is readable in the status bar <b>and in the window title</b>, so in "
         "your system's task bar."),

        ("h1", "If you come from gdb or pwndbg"),
        ("p",
         "namsdec is not a debugger: it simulates the processor instead of driving one. "
         "But the gestures you make in front of a binary are the same, and here they "
         "carry the same names as elsewhere."),
        ("table", [
         ["pwndbg / gdb", "namsdec"],
         ["telescope", "Memory tab, Pointers reading; and every stack word"],
         ["vmmap", "Memory tab › Memory map"],
         ["search", "Memory tab › Search memory"],
         ["patch", "Memory tab › Write here"],
         ["set $rax = …", "click the register value"],
         ["setflag ZF 1", "click the flag"],
         ["cyclic", "Tools › Cyclic pattern (identical to pwntools)"],
         ["dumpargs", "the banner, in front of a call"],
         ["retaddr", "Stack tab › Return addresses"],
         ["nextcall, nextret", "Tools › Until the next… (Space u c / r / s)"],
         ["context", "Edit › Copy state"],
         ["checksec", "Analysis window › Hardening"],
         ["got, plt", "Analysis window › Imported functions"],
         ["nearpc", "the listing itself, RIP in the gutter"],
         ["stepi, nexti, reverse-stepi", "Space s i, Space n i, Space p b"],
        ]),
        ("p",
         "The cyclic pattern is pwntools', byte for byte, so what you measure here is "
         "comparable with what you measure there. <b>Edit › Copy state</b> writes the "
         "registers, the flags, the top of the stack and the next instruction as text, "
         "ready to paste into a report or a message."),
    ]


def _depannage():
    """Ce qui va mal, et ce que namsdec en dit."""
    return [
        ("h1", "When something goes wrong"),
        ("p",
         "Three kinds of trouble, and namsdec answers each in its own place: the "
         "assembler refuses, the program faults, or it runs and gives the wrong answer."),

        ("h2", "nasm refuses"),
        ("p",
         "The <b>Diagnostics</b> tab of the console lists what nasm and "
         "<font face='Courier'>ld</font> said, each message on its line, and a click "
         "takes you to it. The two commands actually run are shown there as well, ready "
         "to copy: what namsdec assembled is what your own command line would have "
         "assembled, and you can check that yourself."),

        ("h2", "What nasm does not tell you"),
        ("p",
         "Two mistakes get through assembly without a word and only show up at run time "
         "— or not at all. namsdec re-reads the linked program and reports them as "
         "<b>warnings</b>, with their line:"),
        ("liste", [
         "<b>A program that nothing stops.</b> No <font face='Courier'>hlt</font>, no "
         "<font face='Courier'>exit</font>: execution carries on into whatever follows "
         "your code, which is data, which is not instructions.",
         "<b>A write to a fixed address</b> that lands in a read-only region — the "
         "classic result of putting a variable in <font face='Courier'>.text</font> "
         "instead of <font face='Courier'>.data</font>.",
        ]),

        ("h2", "The program faults"),
        ("p",
         "The message names the address and the reason, and then says what to do: "
         "\"writing to 0x400000 is forbidden: it is in .text\", and underneath, <i>code "
         "and constants are read-only; what changes during execution goes in section "
         ".data</i>. A fault that only says <font face='Courier'>SIGSEGV</font> teaches "
         "nothing to somebody who is learning."),
        ("note",
         "Execution stops <b>before</b> the faulty instruction, not after: the machine is "
         "in the state that caused the fault, and every register still holds what it held "
         "when the decision was made. <i>Step back</i> works from there like anywhere "
         "else."),

        ("h2", "The answer is wrong"),
        ("p",
         "That is what the workshop is for. Three habits help more than any tool: read "
         "the <b>flags</b> after a <font face='Courier'>cmp</font> rather than guessing "
         "them; hover a register to find out <b>which step wrote it</b>; and use "
         "<b>step back</b> to return to just before the divergence instead of running "
         "the whole program again."),
        ("p",
         "If a conditional jump goes the wrong way, the banner says the rule it followed "
         "— <font face='Courier'>CF = 0 and ZF = 0</font> — and whether it read its "
         "numbers as signed. <font face='Courier'>ja</font> and "
         "<font face='Courier'>jg</font> both say \"greater\" and answer the opposite on "
         "the same bytes; that one line settles the question."),

        ("h2", "Patches, and undoing them"),
        ("p",
         "Everything written into a loaded binary — a byte in the editor, a detour — is "
         "recorded in <b>Tools › Patches</b>: what was changed, where, from what to what. "
         "Each is undone on its own, and a detour is undone as one group, down to the "
         "byte. <b>Nothing touches the disk</b> unless you save, to a file you name."),
    ]


def _encodage():
    """Comment une instruction est faite, et pourquoi ça se voit."""
    return [
        ("h1", "How an instruction is built"),
        ("p",
         "An x86-64 instruction is not a word: it is a sequence of parts, most of them "
         "optional, and the <b>Bytes</b> tab names each one for the instruction under "
         "the cursor. Knowing the shape explains things that otherwise look arbitrary."),
        ("table", [
         ["part", "size", "what it carries"],
         ["prefixes", "0–4 bytes", "operand size, repetition, segment, lock"],
         ["REX", "0 or 1", "64-bit operands, and the registers numbered 8 to 15"],
         ["opcode", "1–3", "the operation itself"],
         ["ModRM", "0 or 1", "which registers, and whether memory is involved"],
         ["SIB", "0 or 1", "base, index and scale of an address"],
         ["displacement", "0, 1, 2 or 4", "the constant added to the address"],
         ["immediate", "0, 1, 2, 4 or 8", "a literal value"],
        ]),
        ("p",
         "<b>REX is why <font face='Courier'>r8</font> costs one byte more than "
         "<font face='Courier'>rax</font>.</b> The eight original registers fit in the "
         "three bits ModRM gives them; the eight numbered ones need a fourth, and that "
         "bit lives in the REX prefix. The same instruction, one byte longer, for no "
         "other reason than the history of the architecture."),
        ("p",
         "<b>And why <font face='Courier'>mov rax, 0</font> is five bytes and not "
         "seven.</b> Writing a 32-bit register clears the top half of the 64-bit one — "
         "that is the rule, not an accident — so the assembler emits "
         "<font face='Courier'>mov eax, 0</font>, which needs no REX and a four-byte "
         "immediate instead of eight. The Bytes tab shows the substitution, and the "
         "disassembly shows <font face='Courier'>eax</font> where you wrote "
         "<font face='Courier'>rax</font>."),
        ("note",
         "This is also why the same mnemonic can have several encodings, and why two "
         "programs that do the same thing can differ by a hundred bytes. When you are "
         "chasing a size, the Bytes tab is where the answer is."),

        ("h2", "Where instructions begin"),
        ("p",
         "There is no mark in the file saying \"an instruction starts here\". Decoding "
         "is sequential: the first byte decides how many follow. Start one byte late and "
         "every instruction after it is different — which is exactly what "
         "anti-disassembly exploits, and what the <b>Code shapes</b> panel shows as two "
         "readings side by side."),
    ]


def _reglages():
    """Préférences, et ce qui s'ajoute sans recompiler."""
    return [
        ("h1", "Settings"),
        ("p",
         "Preferences has four pages. Everything you change there is written to "
         "<font face='Courier'>~/.config/namsdec/settings.json</font> straight away, and "
         "found again on the next launch."),
        ("table", [
         ["page", "what you set there"],
         ["General", "level, language, theme, window decoration, tooltips, completion, "
                     "auto-indent, auto-pairs, instruction help on hover, pre-releases, "
                     "what to do with the previous session"],
         ["Shortcuts", "every action, both sets side by side, rebinding by capture"],
         ["Modules", "which analysis panels are on; predict-before-you-step; "
                     "decompilation and tool fetching, both off by default"],
         ["Plugins", "themes and external tools found on the disk"],
        ]),

        ("h2", "Beginner or advanced"),
        ("p",
         "The level is the one setting that changes the shape of the window rather than "
         "a detail of it. <b>Beginner</b> gives the Workbench: four fixed bands, no "
         "modes in the editor, three disassembly engines instead of six, and the answer "
         "waiting for the step. <b>Advanced</b> gives the full interface. Nothing is "
         "lost either way — the help, the preferences and the exercises are the same at "
         "both levels. A mode that lost them would not be simple, it would be amputated."),

        ("h2", "The two shortcut sets, side by side"),
        ("p",
         "Preferences › Shortcuts shows both, action by action: the active one in full, "
         "the other to compare against, \"same\" where they do not differ. That is how "
         "you see what you came for — the gaps, what you moved yourself, and <b>two "
         "actions that would answer the same keystroke</b>, which it refuses to hide."),
        ("p",
         "Rebinding is done by capture: press the keys you want. A sequence — "
         "<font face='Courier'>Space p r</font> — is captured the same way, key after "
         "key. Switching sets keeps what you moved yourself: it answers \"where do I "
         "start from\", not \"erase what I set\"."),

        ("h1", "Plugins"),
        ("p",
         "Two families, and the distinction is not a ranking but a matter of trust."),
        ("liste", [
         "<b>Themes</b> — colours, and nothing else. A theme does not execute. A JSON "
         "file in <font face='Courier'>~/.local/share/namsdec/themes/</font>.",
         "<b>External tools</b> — the description of a <b>program</b> to run: "
         "<font face='Courier'>objdump</font>, <font face='Courier'>ndisasm</font>, "
         "<font face='Courier'>radare2</font>, Rizin + rz-ghidra, RetDec, or yours. A "
         "JSON file in <font face='Courier'>~/.local/share/namsdec/modules/</font>, with "
         "its installation instructions and the arguments that work.",
        ]),
        ("note",
         "namsdec <b>never goes looking for a program on its own</b>, runs one only on "
         "an explicit request, and writes the full command out before running it. The "
         "repository's <font face='Courier'>plugins/</font> directory holds working "
         "examples of both families."),

        ("h1", "Shortcuts"),
        ("p",
         "The table below is read from the program itself, not copied: it is the active "
         "set as namsdec ships it. Preferences › Shortcuts shows the same list, with the "
         "other set beside it, and lets you change any line."),
    ]
