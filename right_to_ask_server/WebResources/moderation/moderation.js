"use strict";

let current_question_id = null;
let current_question_version = null;
let current_question_num_flags = null;

function resolveCensorship(allow,answers) {
    const command = {
        reason : allow?null:document.getElementById("CensorReason").value,
        censor_logs : true,
        just_answer : answers,
        question_id : current_question_id,
        version : current_question_version,
        num_flags : current_question_num_flags,
    };
    function success(result) {
        console.log(result);
        if (result.Ok) {
            status("Censored question successfully. Bulletin Board hash "+result.Ok);
            updateQuestion();
            refreshQuestions();
        } else {
            status("Tried to censor question. Got Error message "+result.Err);
        }
    }
    if (command.question_id.length!==64) {
        status("Question ID should be 64 hex characters.");
    } else {
        getWebJSON("censor_question",success,failure,JSON.stringify(command),"application/json")
    }
}


function doCensorQuestion() { resolveCensorship(false,[]); }

let censored_answers = [];
function doCensorAnswers() { resolveCensorship(false,censored_answers); }
function doAllowQuestion() { resolveCensorship(true,[]); }

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
    current_question_num_flags = report_reasons.num_flags;
    add(reportDiv,"div","QuestionNumFlags").innerText=report_reasons.num_flags;
    add(reportDiv,"div","QuestionCensorshipStatus QuestionCensorshipStatus_"+report_reasons.censorship_status).innerText=report_reasons.censorship_status;
    const table = add(reportDiv,"table","striped");
    const headline = add(add(table,"thead"),"tr");
    add(headline,"th").innerText="Reason";
    add(headline,"th").innerText="Count";
    add(headline,"th").innerText="Answer";
    const tbody = add(table,"tbody");
    function answer_of_version(version) {
        if (version && question.Ok && question.Ok.answers) {
            const found = question.Ok.answers.find(a=>a.version===version);
            if (found) return found.answer;
        }
        return null;
    }
    for (const reason of report_reasons.reasons) {
        const row = add(tbody,"tr");
        add(row,"td").innerText=reason.reason;
        add(row,"td").innerText=""+reason.count;
        add(row,"td").innerText=answer_of_version(reason.answer) || "";
    }
    document.getElementById("CensorQuestion").disabled=false;
    document.getElementById("Allow").disabled=false;
}
// Called whenever the question ID being investigated changes.
function updateQuestion() {
    let question_id = document.getElementById("QuestionID").value;
    // console.log(question_id);
    const mainDiv = document.getElementById("QuestionDetails");
    removeAllChildElements(mainDiv);
    const infoDiv = add(mainDiv,"div");
    add(mainDiv,"h5").innerText="History";
    const historyDiv = add(mainDiv,"div");
    add(mainDiv,"h5").innerText="Report reasons";
    const reportDiv = add(mainDiv,"div");
    document.getElementById("CensorQuestion").disabled=true;
    document.getElementById("Allow").disabled=true;
    document.getElementById("CensorJustAnswers").disabled=true;
    getWebJSON(getURL("../get_question",{question_id:question_id}),function(question) {
        pretty_show_question(infoDiv,question);
        censored_answers = [];
        if (question.Ok) {
            current_question_id = question.Ok.question_id;
            current_question_version = question.Ok.version;
            current_question_num_flags = 0; // will be modified by get_reasons_reported
            if (question.Ok.answers) {
                for (const answer of question.Ok.answers) {
                    const answerbox = document.getElementById("answer_"+answer.version);
                    if (answerbox) {
                        const cb = add(answerbox,"input");
                        cb.type="checkbox";
                        cb.id="answer_cb_"+answer.version;
                        cb.onclick = function () {
                            censored_answers=censored_answers.filter(v=>v!==answer.version);
                            if (cb.checked) censored_answers.push(answer.version);
                            document.getElementById("CensorJustAnswers").disabled=censored_answers.length===0;
                        }
                        const label = add(answerbox,"label");
                        label.for=cb.id;
                        label.innerText="Censor";
                    }
                }
            }
            getWebJSON(getURL("../get_question_history",{question_id:question_id}),function(history) {
                if (current_question_id === question_id) pretty_show_history(historyDiv,question,history);
            },failure);
            getWebJSON(getURL("get_reasons_reported",{question_id:question_id}),function(report_reasons) {
                if (current_question_id === question_id) pretty_show_report_reasons(reportDiv,question,report_reasons);
            },failure);
        }
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
