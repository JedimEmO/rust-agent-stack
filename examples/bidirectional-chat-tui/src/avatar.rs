use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

// Different cat faces for animation frames
const CAT_FRAMES: &[&[&str]] = &[
    // Frame 1 - Normal
    &[
        " /\\_/\\  ",
        "( o.o ) ",
        " > ^ <  ",
    ],
    // Frame 2 - Blinking
    &[
        " /\\_/\\  ",
        "( -.o ) ",
        " > ^ <  ",
    ],
    // Frame 3 - Winking
    &[
        " /\\_/\\  ",
        "( o.- ) ",
        " > ^ <  ",
    ],
    // Frame 4 - Happy
    &[
        " /\\_/\\  ",
        "( ^.^ ) ",
        " > ^ <  ",
    ],
];

// Different cat variations based on username hash
const CAT_VARIATIONS: &[&[&str]] = &[
    // Classic cat
    &[
        " /\\_/\\  ",
        "( o.o ) ",
        " > ^ <  ",
    ],
    // Chubby cat
    &[
        " /\\_/\\  ",
        "( o.o ) ",
        " (> <)  ",
    ],
    // Sleepy cat
    &[
        " /\\_/\\  ",
        "( -.-)  ",
        " > ^ <  ",
    ],
    // Alert cat
    &[
        " /|_|\\  ",
        "( O.O ) ",
        " > ^ <  ",
    ],
    // Cool cat
    &[
        " /\\_/\\  ",
        "( ■.■ ) ",
        " > ^ <  ",
    ],
    // Surprised cat
    &[
        " /\\_/\\  ",
        "( O.O ) ",
        " > o <  ",
    ],
];

pub struct AvatarManager {
    user_avatars: HashMap<String, usize>,
    frame_counter: usize,
}

impl AvatarManager {
    pub fn new() -> Self {
        Self {
            user_avatars: HashMap::new(),
            frame_counter: 0,
        }
    }

    pub fn tick(&mut self) {
        self.frame_counter = (self.frame_counter + 1) % 40; // Animation cycle
    }

    pub fn get_avatar_for_user(&mut self, username: &str) -> Vec<String> {
        // Assign a consistent avatar variation to each user based on their username
        let avatar_index = self.user_avatars.get(username).copied().unwrap_or_else(|| {
            let mut hasher = DefaultHasher::new();
            username.hash(&mut hasher);
            let hash = hasher.finish();
            let index = (hash % CAT_VARIATIONS.len() as u64) as usize;
            self.user_avatars.insert(username.to_string(), index);
            index
        });

        // Get base avatar
        let base_avatar = CAT_VARIATIONS[avatar_index];
        
        // Determine animation frame based on counter
        let animated_avatar = if self.frame_counter < 30 {
            // Normal face most of the time
            base_avatar
        } else if self.frame_counter < 35 {
            // Blink or wink occasionally
            if avatar_index % 2 == 0 {
                CAT_FRAMES[1] // Blink
            } else {
                CAT_FRAMES[2] // Wink
            }
        } else {
            // Happy face sometimes
            CAT_FRAMES[3]
        };

        // Convert to Vec<String> for easier manipulation
        animated_avatar.iter().map(|&line| line.to_string()).collect()
    }

    pub fn get_compact_avatar_for_user(&mut self, username: &str) -> String {
        // For inline display, just show the face part
        let avatar_index = self.user_avatars.get(username).copied().unwrap_or_else(|| {
            let mut hasher = DefaultHasher::new();
            username.hash(&mut hasher);
            let hash = hasher.finish();
            let index = (hash % CAT_VARIATIONS.len() as u64) as usize;
            self.user_avatars.insert(username.to_string(), index);
            index
        });

        // Return just the face line for compact display
        let face = if self.frame_counter < 30 {
            CAT_VARIATIONS[avatar_index][1]
        } else if self.frame_counter < 35 {
            if avatar_index % 2 == 0 {
                CAT_FRAMES[1][1] // Blink
            } else {
                CAT_FRAMES[2][1] // Wink
            }
        } else {
            CAT_FRAMES[3][1] // Happy
        };

        face.to_string()
    }
}