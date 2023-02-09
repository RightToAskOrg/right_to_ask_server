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
    add(id_div,"div").innerText="Last modified  : "+prettyTime(question.last_modified);
    add(id_div,"div").innerText="Created  : "+prettyTime(question.timestamp);
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
    if (question.answer_accepted) {
        add(div,"div").innerText="Question answer accepted!"
    }
    // question.hansard_link, if it exists, is an array of type HansardLink, not well defined yet.
    if (question.is_followup_to) {
        const link = add(div,"a");
        link.innerText="Question to which this is a follow up"; // This is the sort of grammar up with which we should not put.
        link.href = "ShowQuestion.html?question_id="+question.is_followup_to;
    }
    if (question.answers) {
        add(div,"h2").innerText="Answers";
        for (const answer of question.answers) {
            const figure = add(div, "figure");
            add(figure, "blockquote").innerText = answer.answer;
            const caption = add(figure, "figcaption");
            caption.innerText = answer.answered_by + " wearing hat as " + mp_id_tostring(answer.mp) + " time " + prettyTime(answer.timestamp);
            caption.id="answer_"+answer.version; // Needed by moderation for a place to insert the hook to censor this answer.
        }
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
        if (history.Ok && history.Ok.history) {
            const table = add(div,"table","striped");
            const headrow = add(add(table,"thead"),"tr");
            add(headrow,"th").innerText="time";
            add(headrow,"th").innerText="user";
            add(headrow,"th").innerText="action";
            const tbody = add(table,"tbody");
            for (const h of history.Ok.history) { // These are in reverse chronological order.
                const tr = add(tbody,"tr");
                add(tr,"td").innerText=prettyTime(h.timestamp);
                // could have a link to bulletin board via addLink(where,h.id);
                const who_cell = add(tr,"td");
                const action_cell = add(tr,"td");
                if (h.action) {
                    // this is of type LogInBulletinBoard
                    for (const field in h.action) {
                        let command = h.action[field];
                        if (command&&command.command && command.command.user)  who_cell.innerText=command.command.user;
                    }
                    if (h.action.EditQuestion) {
                        action_cell.innerText="Edit Question";
                        const command = h.action.EditQuestion.command;// TODO add actual fields edited.
                        console.log(command);
                    } else if (h.action.NewQuestion) {
                        action_cell.innerText="New Question created";
                        const command = h.action.NewQuestion.command;// TODO add actual fields edited.
                        console.log(command);
                    } else if (h.action.PlainTextVoteQuestion) {
                        action_cell.innerText = "Voted";
                        // should not be stored in history, but were in some old test data.
                    } else if (h.action.CensorQuestion) {
                        action_cell.innerText = "Censorship performed";
                    } else {
                        action_cell.innerText=JSON.stringify(h.action); // what could be prettier? Especially as one field is sometimes a cryptographic signature of a JSON-serialized structure.
                    }
                } else {
                    action_cell.innerText="Content Censored";
                    action_cell.className="Censored"
                }
            }
        }
    }

}

function prettyTime(timestamp) {
    const date = new Date(timestamp*1000);
    return date.toLocaleDateString();
}

function mp_id_tostring(mp) { return mp.first_name+" "+mp.surname+" ("+mp.electorate.chamber+(mp.electorate.region?(" "+mp.electorate.region):"")+")"; }
