//! Prints a new key pair for web push. Put the private key in the backend's
//! `VAPID_PRIVATE_KEY` (Render, or `backend/.env`) and nowhere else; browsers
//! get the public key from `GET /push/key`.
//!
//! Changing the key later stops every phone's notifications until the app
//! subscribes again, which it does the next time it opens.

fn main() {
    let (private, public) = coaching_backend::push::new_keys();
    println!("VAPID_PRIVATE_KEY={private}");
    println!("public key (for reference): {public}");
}
