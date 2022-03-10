"use strict";

function showQuestion(div,user) {
    const a = add(div,"a");
    a.innerText=user;
    a.href = "get_question?question_id="+encodeURIComponent(user);
}

function updateQuestionList() {
    const div = document.getElementById("QuestionList");
    if (div) { // only use for a page with a list of questions.
        function success(data) {
            // console.log(data);
            removeAllChildElements(div);
            if (data.Ok) for (const line of data.Ok) showQuestion(add(div,"div"),line);
            else if (data.Err) div.innerText="Error : "+data.Err;
        }
        getWebJSON("get_question_list",success,failure);
    }
}

function getQuestionNonDefiningFields() {
    let res = {};
    let text = document.getElementById("BackgroundText").value;
    if (text.length>0) res.background = text;
    text = document.getElementById("FollowUpTo").value;
    if (text.length>0) res.is_followup_to = text;
    return res;
}

function addQuestion() {
    function success(result) {
        console.log(result);
        if (result.Ok) {
            check_signature(result.Ok);
            const decoded = JSON.parse(result.Ok.message);
            status("Added question successfully. Question id "+decoded.question_id+" Bulletin Board hash "+decoded.version+" signature "+result.Ok.signature);
        } else {
            status("Tried to add question. Got Error message "+result.Err);
        }
        updateQuestionList();
    }
    let command = getQuestionNonDefiningFields();
    command.question_text = document.getElementById("QuestionText").value;
    let signed_message = sign_message(command);
    getWebJSON("new_question",success,failure,JSON.stringify(signed_message),"application/json")
}

window.onload = function () {
    document.getElementById("Add").onclick = addQuestion;
    updateQuestionList();
}