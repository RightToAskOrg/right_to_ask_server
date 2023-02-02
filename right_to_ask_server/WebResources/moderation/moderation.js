"use strict";


function doCensorQuestion() {

}
function doCensorAnswers() {

}
function doAllowQuestion() {

}

function refreshQuestions() {
    function success(list) {
        document.getElementById("NumQuestions").innerText=""+list.length;
        const questions_div = document.getElementById("QuestionList");
        removeAllChildElements(questions_div);
        for (const question of list) {
            const question_div = add(questions_div,"div","ModerationQuestionDiv");
            const status_div = add(question_div,"div","ModerationQuestionStatusDiv");
            add(status_div,"div","QuestionID").innerText=question.id;
            add(status_div,"div","QuestionNumFlags").innerText=question.num_flags;
            add(status_div,"div","QuestionCensorshipStatus QuestionCensorshipStatus_"+question.censorship_status).innerText=question.censorship_status;
            add(question_div,"div","QuestionText").innerText=question.question_text;
            question_div.onclick = function () {
                document.getElementById("QuestionID").value=question.id;
                updateQuestion();
            }
        }
    }
    getWebJSON("get_reported_questions",function (list) { if (list.hasOwnProperty("Ok")) success(list.Ok); else failure(list.Err); },failure);
}

function pretty_show_report_reasons(reportDiv,question,report_reasons) {
    if (report_reasons.Err) { failure("Report reasons error : "+report_reasons.Err); return; }
    report_reasons=report_reasons.Ok;
    add(reportDiv,"div","QuestionNumFlags").innerText=report_reasons.num_flags;
    add(reportDiv,"div","QuestionCensorshipStatus QuestionCensorshipStatus_"+report_reasons.censorship_status).innerText=report_reasons.censorship_status;
    const table = add(reportDiv,"table","striped");
    const headline = add(add(table,"thead"),"tr");
    add(headline,"th").innerText="Reason";
    add(headline,"th").innerText="Count";
    add(headline,"th").innerText="Answer";
    const tbody = add(table,"tbody");
    for (const reason of report_reasons.reasons) {
        const row = add(tbody,"tr");
        add(row,"td").innerText=reason.reason;
        add(row,"td").innerText=""+reason.count;
        add(row,"td").innerText=reason.answer || "";
    }
    // TODO adjust button enabledness
}
// Called whenever the question ID being investigated changes.
function updateQuestion() {
    let question_id = document.getElementById("QuestionID").value;
    console.log(question_id);
    const mainDiv = document.getElementById("QuestionDetails");
    removeAllChildElements(mainDiv);
    const infoDiv = add(mainDiv,"div");
    add(mainDiv,"h5").innerText="History";
    const historyDiv = add(mainDiv,"div");
    add(mainDiv,"h5").innerText="Report reasons";
    const reportDiv = add(mainDiv,"div");
    getWebJSON(getURL("../get_question",{question_id:question_id}),function(question) {
        pretty_show_question(infoDiv,question);
        getWebJSON(getURL("../get_question_history",{question_id:question_id}),function(history) {
            pretty_show_history(historyDiv,question,history);
        },failure);
        getWebJSON(getURL("get_reasons_reported",{question_id:question_id}),function(report_reasons) {
            pretty_show_report_reasons(reportDiv,question,report_reasons);
        },failure);
    },failure);
}

window.onload = function () {
    document.getElementById("Refresh").onclick = refreshQuestions;
    document.getElementById("CensorQuestion").onclick = doCensorQuestion;
    document.getElementById("CensorJustAnswers").onclick = doCensorAnswers;
    document.getElementById("Allow").onclick = doAllowQuestion;
    document.getElementById("QuestionID").oninput = updateQuestion;
    refreshQuestions();
}
