use std::io;    // I/O from user library
use std::cmp::Ordering; //compare function
use rand::Rng;  //need to have rand="version" in cargo.toml

fn main() {
    println!("Guess the number!");
    let secret_number: u32 = rand::thread_rng().gen_range(1..=100);  //rng, range is inclusive (1..=100 == [1, 2, ..., 99, 100])
    let mut tries: u32 = 0;
    println!("{secret_number}");
    loop{
    println!("Please input your guess.");

    let mut guess = String::new();          //new variable mut=alterable

    io::stdin()
        .read_line(&mut guess)              //reads input and sets the input as guess
        .expect("Failed to read line");     //if read fails this happens

    let guess: u32 = match guess.trim().parse() {      //trim(): remove whitespace(\n). parse(): converts type (srt to u32)
        Ok(num)=>num,                           //success
        Err(_) => continue,                     //fail
    };
    tries+=1;
    //tries=&tries+1;

    println!("try {tries}, You guessed: {guess}");
    match guess.cmp(&secret_number){
        Ordering::Less => println!("number too low"),
        Ordering::Greater => println!("number too high"),
        Ordering::Equal => {println!("got 'em with {tries} tries" );
        break;
        }
    }
}
}