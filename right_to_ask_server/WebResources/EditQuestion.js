"use strict";

function editQuestion() {
    function success(result) {
        console.log(result);
        if (result.Ok) {
            check_signature(result.Ok);
            status("Edited question successfully. Bulletin Board hash "+result.Ok.message+" signature "+result.Ok.signature);
            updateQuestion();
        } else {
            status("Tried to edit question. Got Error message "+result.Err);
        }
    }

    let command = {
        question_id : question_id,
        version : question.version,
    };
    let background = document.getElementById("BackgroundText").value;
    if (background!==question.background||"") { command.background = background };
    let is_followup_to = document.getElementById("FollowUpTo").value;
    if (is_followup_to!==(question.is_followup_to||"")) { command.is_followup_to = is_followup_to };
    if (document.getElementById("PermissionsAsk").checked!==(question.who_should_ask_the_question_permissions==="WriterOnly")) {
        command.who_should_ask_the_question_permissions = document.getElementById("PermissionsAsk").checked?"WriterOnly":"Others";
    }
    if (document.getElementById("PermissionsAnswer").checked!==(question.who_should_answer_the_question_permissions==="WriterOnly")) {
        command.who_should_answer_the_question_permissions = document.getElementById("PermissionsAnswer").checked?"WriterOnly":"Others";
    }
    let signed_message = sign_message(command);
    getWebJSON("edit_question",success,failure,JSON.stringify(signed_message),"application/json")
}

let question_id = null;
let question = null;

function setQuestion(questionInfo) {
    console.log(questionInfo);
    if (questionInfo.Ok) {
        question = questionInfo.Ok;
        document.getElementById("QuestionID").innerText=question_id;
        document.getElementById("CreatedTime").innerText=question.timestamp;
        document.getElementById("LastModified").innerText=question.last_modified;
        document.getElementById("QuestionText").innerText=question.question_text;
        document.getElementById("Author").innerText=question.author;
        document.getElementById("Version").innerText=question.version;
        document.getElementById("BackgroundText").value=question.background||"";
        document.getElementById("FollowUpTo").value=question.is_followup_to||"";
        document.getElementById("PermissionsAsk").checked=question.who_should_ask_the_question_permissions==="WriterOnly";
        document.getElementById("PermissionsAnswer").checked=question.who_should_answer_the_question_permissions==="WriterOnly";
    } else if (questionInfo.Err) failure("Error : "+questionInfo.Err);
}

function updateQuestion() {
    getWebJSON(getURL("get_question",{question_id:question_id}),setQuestion,failure);
}
window.onload = function () {
    question_id = new URLSearchParams(window.location.search).get("question_id");
    document.getElementById("Edit").onclick = editQuestion;
    updateQuestion();
}