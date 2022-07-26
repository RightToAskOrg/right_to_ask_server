"use strict";

/* This is designed to just be used in the ShowQuestion.html page */

window.onload = function () {
    const question_id = new URLSearchParams(window.location.search).get("question_id");
    getWebJSON(getURL("get_question",{question_id:question_id}),function(question) {
        pretty_show_question(document.getElementById("QuestionDescription"),question);
        getWebJSON(getURL("get_question_history",{question_id:question_id}),function(history) {
            pretty_show_history(document.getElementById("HistoryDescription"),question,history);
        },failure);
    },failure);
}
