use crate::domain::participant::Participant;


pub fn name_to_initials(name: &str) -> String {
    let mut initials = String::new();
    for word in name.split_whitespace() {
        initials.push(word.chars().next().unwrap());
        initials.push('.');
    }
    initials
}


pub fn get_participant_public_name(participant: &Participant) -> String {
    if participant.is_anonymous {
       name_to_initials(&participant.name)
    } else {
        participant.name.clone()
    }
}

pub fn get_participant_model_public_name(participant: &crate::schema::participant::Model) -> String {
    if participant.is_anonymous {
        name_to_initials(&participant.name)
    } else {
        participant.name.clone()
    }
}