"use strict";

/*

 Code to display a question, or a history, in some div, in a pretty way, suitable for end users.

 This is designed to be used in multiple places (e.g. in the public facing site, and also in various admin sites such as censorship administration).
 */


/**
 * Pretty display the question in the div
 * @param div An html DOM element to display in. Typically a div.
 * @param question Information on the question. In the format in the server Result<QuestionInfo> structure.
 */
function pretty_show_question(div,question) {
    console.log(question);
    removeAllChildElements(div);
    if (question.Err) { // deal with being unable to get the question.
        add(div,"div","Error").innerText="Error : "+question.Err;
        return;
    } else { question=question.Ok; }
    add(div,"div","QuestionText").innerText = question.question_text;
    const id_div = add(div,"div");
    add(id_div,"h2").innerText="Identity";
    add(id_div,"div").innerText="Question id : "+question.question_id;
    add(id_div,"div").innerText="Version  : "+question.version;
    add(id_div,"div").innerText="Last modified  : "+question.last_modified+" seconds since 1970-01-01 00:00:00 UTC"
    add(id_div,"div").innerText="Created  : "+question.timestamp+" seconds since 1970-01-01 00:00:00 UTC"
    add(div,"span").innerText="Author : "+question.author;
    if (question.background) {
        const background = add(div,"QuestionBackground");
        add(background,"h5").innerText="Background";
        background.append(question.background);
    }
    if (question.mp_who_should_ask_the_question) {
        // TODO This is an array of PersonID objects
    }
    if (question.entity_who_should_answer_the_question) {
        // TODO This is an array of PersonID objects
    }
    // TODO question.who_should_ask_the_question_permissions : Permissions,
    // TODO question.entity_who_should_answer_the_question : Permissions,
    if (question.answers) for (const answer of question.answers) {
        // TODO answer is of type QuestionAnswer, see question.rs for fields
    }
    if (question.answer_accepted) {
        add(div,"div").innerText="Question answer accepted!"
    }
    // question.hansard_link, if it exists, is an array of type HansardLink, not well defined yet.
    if (question.is_followup_to) {
        const link = add(div,"a");
        link.innerText="Question to which this is a follow up"; // This is the sort of grammar up with which we should not put.
        link.href = "ShowQuestion.html?question_id="+question.is_followup_to;
    }
}

/**
 * Pretty display the history in the div
 * @param div An html DOM element to display in. Typically a div.
 * @param question Information on the question. In the format in the server QuestionInfo structure. May be useful for context.
 * @param history The history to show. Result<QuestionHistory> from censorship.rs
 */
function pretty_show_history(div,question,history) {
    removeAllChildElements(div);
    add(div,"h2").innerText="History";
    if (history.Err) { // deal with being unable to get the question.
        add(div,"div","Error").innerText="Error : "+history.Err;
        return;
    } else {
        if (history.Ok && history.Ok.history) for (const h of history.Ok.history) { // These are in reverse chronological order.
            const bb = add(div,"div");
            add(bb,"span").innerText="Bulletin board : ";
            addLink(bb,h.id);
            add(div,"div").innerText="Timestamp : "+h.timestamp;
            if (h.action) {
                // this is of type LogInBulletinBoard
                let action = add(div,"div");
                action.innerText=JSON.stringify(h.action); // what could be prettier? Especially as one field is sometimes a cryptographic signature of a JSON-serialized structure.
            } else add(div,"span","censored").innerText="Censored";
        }
    }

}
