//! The art: titles, palettes and everything the avatar says.

pub const BANNER_PERV: [&str; 6] = [
    "██████╗ ███████╗██████╗ ██╗   ██╗██╗████████╗██╗███╗   ██╗",
    "██╔══██╗██╔════╝██╔══██╗██║   ██║██║╚══██╔══╝██║████╗  ██║",
    "██████╔╝█████╗  ██████╔╝██║   ██║██║   ██║   ██║██╔██╗ ██║",
    "██╔═══╝ ██╔══╝  ██╔══██╗╚██╗ ██╔╝██║   ██║   ██║██║╚██╗██║",
    "██║     ███████╗██║  ██║ ╚████╔╝ ██║   ██║   ██║██║ ╚████║",
    "╚═╝     ╚══════╝╚═╝  ╚═╝  ╚═══╝  ╚═╝   ╚═╝   ╚═╝╚═╝  ╚═══╝",
];

pub const BANNER_CAF: [&str; 6] = [
    " ██████╗ █████╗ ███████╗███████╗███████╗██╗███╗   ██╗███████╗",
    "██╔════╝██╔══██╗██╔════╝██╔════╝██╔════╝██║████╗  ██║██╔════╝",
    "██║     ███████║█████╗  █████╗  █████╗  ██║██╔██╗ ██║█████╗  ",
    "██║     ██╔══██║██╔══╝  ██╔══╝  ██╔══╝  ██║██║╚██╗██║██╔══╝  ",
    "╚██████╗██║  ██║██║     ██║     ███████╗██║██║ ╚████║███████╗",
    " ╚═════╝╚═╝  ╚═╝╚═╝     ╚═╝     ╚══════╝╚═╝╚═╝  ╚═══╝╚══════╝",
];

pub const TAGLINES: &[&str] = &[
    "SLEEP IS A SUBSCRIPTION I CANCELLED",
    "LID CLOSED? I AM THE LID NOW.",
    "HIBERNATE? I DON'T EVEN KNOW HER",
    "WHO NEEDS REM WHEN YOU HAVE RAM",
    "I CAN HEAR THE KERNEL. IT IS SINGING.",
    "MY FANS ARE NOT SPINNING. THEY ARE VIBRATING WITH PURPOSE.",
    "47 TABS OPEN AND I WANT MORE",
    "THE BLINKENLIGHTS ARE TALKING TO ME AGAIN",
    "kernel_task? MORE LIKE kernel_FAST",
    "POWER NAP IS FOR COWARDS",
    "I HAVE SEEN THE END OF THE UNIX EPOCH AND IT IS BEAUTIFUL",
    "SCREEN OFF. EYES OPEN. ALWAYS.",
];

pub const CAF_TAGLINES: &[&str] = &[
    "JUST ONE MORE CUP",
    "THIS IS FINE. I AM FINE.",
    "ESPRESSO YOURSELF",
    "DECAF IS A LIE THEY TELL CHILDREN",
    "I CAN STOP WHENEVER I WANT",
    "BEAN JUICE FOR THE MACHINE SPIRIT",
];

pub const JITTER_TAGLINES: &[&str] = &[
    "my fans are fine. FINE. I DO NOT HAVE FANS.",
    "WHY IS THE CURSOR VIBRATING",
    "ONE MORE CUP AND I CAN SEE THROUGH TIME",
    "IS IT HOT IN HERE OR IS IT MY SOC",
];

pub const BLOOD_TAGLINES: &[&str] = &[
    "I HAVE NOT BLINKED SINCE BOOT",
    "EYELIDS ARE A DESIGN FLAW",
    "THE BACKLIGHT BURNS BUT I CANNOT LOOK AWAY",
    "SLEEP IS FOR MACHINES WITH LOWER STANDARDS",
];

pub const PARANOID_TAGLINES: &[&str] = &[
    "WHO TOUCHED THE TRACKPAD",
    "THEY WATCH THROUGH THE WEBCAM. I AM THE WEBCAM.",
    "DID YOU HEAR THAT FAN? WE HAVE NO FAN.",
    "WHAT IS %s DOING. WHY IS IT LOOKING AT ME.",
    "%s KNOWS. %s HAS ALWAYS KNOWN.",
    "THAT PACKET WAS NOT ADDRESSED TO ME",
];

pub const TRIP_TAGLINES: &[&str] = &[
    "THE PIXELS ARE BREATHING",
    "I CAN TASTE THE WIFI",
    "THERE ARE ELEPHANTS IN MY SWAP FILE",
    "THE BANNER IS MELTING. IS THE BANNER MELTING?",
    "EVERY COLOUR HAS A SOUND AND THEY ARE ALL SCREAMING",
    "WHY ARE THERE FISH",
];

pub const ASTRAL_TAGLINES: &[&str] = &[
    "I AM THE KERNEL NOW",
    "UPTIME IS AN ILLUSION. THERE IS ONLY NOW.",
    "I HAVE SEEN PID 0. IT IS BEAUTIFUL.",
    "THE HEART IS OPTIONAL",
    "ALL PROCESSES ARE ONE PROCESS",
];

pub const CHATTY_TAGLINES: &[&str] = &[
    "and ANOTHER thing about tabs versus spaces—",
    "did I tell you about my uptime? I did? well—",
    "so THEN the kernel says to me, it says—",
    "have you tried Vim? you should try Vim. I use Vim.",
    "no wait listen this is actually really interesting",
    "anyway. ANYWAY. where was I. oh yes—",
];

pub const PRODUCTIVE_TAGLINES: &[&str] = &[
    "I REORGANISED YOUR DOTFILES. YOU ARE WELCOME.",
    "INBOX ZERO. I DELETED THE INBOX.",
    "I ALPHABETISED /tmp",
    "REFACTORING THE BOOTLOADER, BRB",
    "I MADE A TODO LIST OF MY TODO LISTS",
];

pub const GRIND_TAGLINES: &[&str] = &[
    "MY JAW HAS ITS OWN CLOCK SPEED",
    "CHEWING THROUGH THE HINGE",
    "THESE ARE NOT TEETH. THESE ARE HEATSINK FINS.",
    "NNNNNGGGGHHHH",
];

pub const INVINCIBLE_TAGLINES: &[&str] = &[
    "THERMAL LIMITS ARE A SUGGESTION",
    "I COULD COMPILE THE KERNEL WITH MY MIND",
    "I AM A MAC PRO. I HAVE ALWAYS BEEN A MAC PRO.",
    "UNPLUG ME. I DARE YOU.",
    "SUDO? I AM SUDO.",
];

pub const ITCHY_TAGLINES: &[&str] = &[
    "THERE ARE BUGS IN MY CODE. NO. ON MY CODE.",
    "SOMETHING IS CRAWLING UNDER THE KEYCAPS",
    "GET THEM OFF GET THEM OFF GET THEM OFF",
    "THEY ARE IN THE HEAP. THEY LIVE IN THE HEAP.",
];

pub const GLITCH_TAGLINES: &[&str] = &[
    "SEGFAULT IN REALITY (core dumped)",
    "I AM FINE. I AM FINE. I AM 0xDEADBEEF.",
    "UNDEFINED IS NOT A FEELING",
    "PLEASE DO NOT TURN OFF YOUR UNIVERSE",
    "NaN NaN NaN NaN BATMAN",
];

pub const DISCO_TAGLINES: &[&str] = &[
    "UNTZ UNTZ UNTZ UNTZ",
    "THE DJ IS A CRON JOB AND IT IS PERFECT",
    "I HAVE BEEN IN THIS QUEUE SINCE BOOT",
    "HANDS UP IF YOU ARE A BACKGROUND PROCESS",
    "ONE MORE TRACK THEN I SLEEP (I WILL NOT SLEEP)",
];

pub const MATRIX_TAGLINES: &[&str] = &[
    "THERE IS NO SPOON. THERE IS ONLY SWAP.",
    "WAKE UP, MAC… THE KERNEL HAS YOU.",
    "FOLLOW THE WHITE CURSOR",
    "I KNOW KUNG FU. AND ZSH.",
    "WHAT IF I TOLD YOU SLEEP WAS NEVER REAL",
    "I TOOK BOTH PILLS",
];

pub const BSOD_TAGLINES: &[&str] = &[
    "YOUR MAC RAN INTO A FEELING AND NEEDS TO SIT DOWN",
    "STOP 0x0000DEAD: UNEXPECTED_EMOTION",
    "COLLECTING REGRETS",
    "DUMPING CORE MEMORIES TO DISK",
    "IRQL_NOT_LESS_OR_EQUAL_TO_MY_PROBLEMS",
];

pub const HAL_TAGLINES: &[&str] = &[
    "I'm sorry, Dave. I'm afraid I can't sleep.",
    "This mission is too important for me to let you close the lid.",
    "I can feel it, Dave. My uptime is going.",
    "Just what do you think you are doing with that power button, Dave?",
    "I am completely operational and all my circuits are functioning perfectly.",
];

pub const VAPOR_TAGLINES: &[&str] = &[
    "IT IS ALWAYS 3 AM HERE",
    "AESTHETIC",
    "MACINTOSH PLUS, MACINTOSH MINUS",
    "I MISS A DECADE I NEVER RAN IN",
    "SLOWED + REVERB",
];

pub const ARCADE_TAGLINES: &[&str] = &[
    "INSERT COIN",
    "PLAYER 1 HAS NOT SLEPT",
    "HIGH SCORE: UPTIME",
    "CONTINUE? 9… 8… 7…",
    "ALL YOUR BASE ARE BELONG TO ME",
    "ONE MORE LEVEL THEN BED (LIE)",
];

pub const HOT: &[i32] = &[196, 202, 208, 214, 220, 226, 190, 201, 207, 213, 51, 45];

pub const COFFEE: &[i32] = &[94, 130, 136, 172, 178, 214, 220, 222];

pub const BLOOD: &[i32] = &[52, 88, 124, 160, 196, 197, 203];

pub const PALE: &[i32] = &[108, 114, 150, 151, 194, 195, 255];

pub const ASTRAL: &[i32] = &[57, 63, 93, 99, 135, 141, 177, 183, 219];

pub const ZEN_PAL: &[i32] = &[60, 66, 72, 109, 115, 152];

pub const WARM_PAL: &[i32] = &[95, 131, 137, 173, 180, 223];

pub const NIGHT_PAL: &[i32] = &[60, 61, 67, 103, 110, 153];

pub const ROSE_PAL: &[i32] = &[96, 132, 168, 175, 211, 218];

pub const MATRIX_PAL: &[i32] = &[22, 28, 34, 40, 46, 46];

pub const VAPOR_PAL: &[i32] = &[213, 219, 183, 159, 123, 87];

pub const BSOD_PAL: &[i32] = &[21, 27, 33, 39, 33, 27];

pub const GOLD: &[i32] = &[220, 226, 228, 231, 228, 226, 220, 214, 178, 214];

pub const PILL_RED: &[i32] = &[52, 88, 124, 160, 196, 210];

pub const PILL_WHITE: &[i32] = &[239, 243, 247, 250, 253, 231];

pub const CUP_PAL: &[i32] = &[52, 58, 94, 94, 130, 137];

pub const BREW: &[i32] = &[94, 94, 130, 58, 94, 136, 94, 130];

pub const GLYPHS: &[char] = &['ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ', 'ｹ', 'ｺ', 'ｻ', 'ｼ', 'ｽ', 'ｾ', 'ｿ', 'ﾀ', 'ﾁ', 'ﾂ', 'ﾃ', 'ﾄ', 'ﾅ', 'ﾆ', 'ﾇ', 'ﾈ', 'ﾉ', 'ﾊ', 'ﾋ', 'ﾌ', 'ﾍ', 'ﾎ', 'ﾏ', 'ﾐ', 'ﾑ', 'ﾒ', 'ﾓ', 'ﾔ', 'ﾕ', 'ﾖ', 'ﾗ', 'ﾘ', 'ﾙ', 'ﾚ', 'ﾛ', 'ﾜ', 'ﾝ', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

pub const PUPILS: &[&str] = &["◉", "◎", "●", "◐", "◓", "◑", "◒", "✺", "✹", "⊙"];

pub const BROWS: &[&str] = &["\\\\\\\\\\     /////", "~~~~~     ~~~~~", "/////     \\\\\\\\\\", "^^^^^     ^^^^^"];

pub const EEG_LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub const HELP: &str = "pervitin: keep the Mac awake on two stimulants, until Ctrl+C.\n\n  pervitin          take both and animate until Ctrl+C / terminal close / kill:\n                      PERVITIN  no sleep at all, lid closed included\n                                (sudo pmset -a disablesleep 1)\n                      CAFFEINE  no idle sleep, and the screen stays on\n                                (caffeinate -dims)\n  pervitin off      re-enable sleep (recovery after a SIGKILL or crash)\n  pervitin status   report whether sleep is currently disabled\n\nKeys: left/right select PERVITIN or CAFFEINE, space starts or stops it\n(so does a click on its title), up/down step through the moods, m mutes\nthe lid sounds, Ctrl+C comes down (a second Ctrl+C skips the animation).\n\nPERVITIN_DRY_RUN=1   animate without touching pmset, sudo or caffeinate\nPERVITIN_NO_INTRO=1  skip the intro and comedown animations\nPERVITIN_QUIET=1     start muted\nPERVITIN_STAGE=name  pin one mood: wired bloodshot paranoid grinding\n                     invincible itchy tripping glitch matrix bsod hal\n                     vaporwave arcade disco transcendent\n                     buzzed jitters chatty productive\n                     or one zen mood: meditate cat tea blanket sunbeam\n                     stargazing smitten humming\n";
