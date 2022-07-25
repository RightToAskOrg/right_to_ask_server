"use strict";

function censorQuestion() {
    function success(result) {
        console.log(result);
        if (result.Ok) {
            status("Censored question successfully. Bulletin Board hash "+result.Ok);
        } else {
            status("Tried to censor question. Got Error message "+result.Err);
        }
    }

    let command = {
        reason : document.getElementById("Reason").value,
        question_id : document.getElementById("QuestionID").value,
        censor_logs : document.getElementById("CensorLogs").checked,
    };
    console.log(command);
    console.log(command.question_id.length);
    if (command.question_id.length!==64) {
        status("Question ID should be 64 hex characters.");
    } else if (!command.reason.length) {
        status("Need to give a reason.");
    } else {
        getWebJSON("censor_question",success,failure,JSON.stringify(command),"application/json")
    }
}

window.onload = function () {
    let question_id = new URLSearchParams(window.location.search).get("question_id");
    if (question_id) document.getElementById("QuestionID").value=question_id;
    document.getElementById("Censor").onclick = censorQuestion;
}
