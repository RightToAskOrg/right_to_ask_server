"use strict";

/** Add a line to the status display.
 * @param line{string} line to add to the status */
function status(line) {
    add(document.getElementById("status"),"div").innerText=line;
}
function failure(error) {
    status("Error : "+error);
}

function updateAllList() {
    /*
    function success(data) {
        if (data.Ok) {
            console.log(data.Ok);
            const div = document.getElementById("AllQuestions");
            removeAllChildElements(div);
            for (const question of data.Ok) {
                add(div,"div","question").innerText=question;
            }
        } else { failure(data.Err) }
    }
    getWebJSON("get_all_questions",success,failure);
     */
}

let current_page_command = null;
function goto_next_page() {
    let command = JSON.parse(JSON.stringify(current_page_command)); // clone
    const questions_per_page = command.page.to-command.page.from;
    command.page.to+=questions_per_page;
    command.page.from+=questions_per_page;
    getWebJSON("get_similar_questions",data => redraw_question_list(command,data),failurePending,JSON.stringify(command),"application/json");
}

function redraw_question_list(command,data) {
    if (data.Err) failure(data.Err);
    else {
        console.log(data.Ok);
        const div = document.getElementById("SimilarQuestions");
        removeAllChildElements(div);
        for (const possibility of data.Ok.questions) {
            let line = add(div,"div","SimilarQuestionLine");
            add(line,"span","score").innerText = possibility.score.toFixed(2);
            function foundQuestion(data) {
                if (data.Ok) line.append(" ["+data.Ok.author+"] "+data.Ok.question_text);
            }
            getWebJSON(getURL("get_question",{question_id:possibility.id}),foundQuestion,failure);
        }
        document.getElementById("current_page_from").innerText=""+command.page.from;
        document.getElementById("current_page_to").innerText=""+Math.min(command.page.to,command.page.from+data.Ok.questions.length);
        const next_page_button = document.getElementById("next_page");
        current_page_command=command;
        if (data.Ok.token) {
            next_page_button.disabled=false;
            current_page_command.page.token=data.Ok.token;
        } else {
            next_page_button.disabled=true;
        }
        pendingCheck();
    }
}
function pendingCheck() {
    currently_pending_check_similarity=false;
    if (should_do_new_check_similarity) {
        should_do_new_check_similarity=false;
        checkSimilarity();
    }
}
function failurePending(message) {
    failure(message);
    pendingCheck();
}

let currently_pending_check_similarity = false;
let should_do_new_check_similarity = false;
function checkSimilarity() {
    if (currently_pending_check_similarity) { should_do_new_check_similarity=true; return; }

    currently_pending_check_similarity=true;
    const command = {question_text:document.getElementById("entry").value};
    function parseUsers(list,ui,tag) {
        for (const s of document.getElementById(ui).value.split(',')) {
            let ss = s.trim();
            if (ss.length>0) {
                let e = {};
                e[tag]=ss;
                list.push(e);
            }
        }
    }
    let askList = addMPsAskList.slice(); // make a shallow copy
    for (const c of addCommitteesAskList) askList.push(c); // append committees.
    let answerList = addMPsAnswerList.slice(); // make a shallow copu
    parseUsers(askList,"AddUserAskList","User");
    parseUsers(answerList,"AddUserAnswerList","User");
    parseUsers(askList,"AddDomainAskList","Organisation");
    parseUsers(answerList,"AddDomainAnswerList","Organisation");
    if (askList.length>0) { command.mp_who_should_ask_the_question = askList; }
    if (answerList.length>0) { command.entity_who_should_answer_the_question = answerList; }
    command.weights = {
        text : +document.getElementById("weight_text").value,
        metadata : +document.getElementById("weight_metadata").value,
        total_votes : +document.getElementById("weight_total_votes").value,
        net_votes : +document.getElementById("weight_net_votes").value,
        recentness : +document.getElementById("weight_recentness").value,
        recentness_timescale : +document.getElementById("weight_recentness_timescale").value,
    };
    command.page = {
        from : +document.getElementById("page_from").value,
        "to" : +document.getElementById("page_to").value
    }
    getWebJSON("get_similar_questions",data => redraw_question_list(command,data),failurePending,JSON.stringify(command),"application/json");
}

let addMPsAskList = [];
let addCommitteesAskList = [];
let addMPsAnswerList = [];

window.onload = function () {
    for (const dynamic_fields of ["entry","weight_text","weight_metadata","weight_total_votes","weight_net_votes","weight_recentness","weight_recentness_timescale","page_from","page_to"]) {
        document.getElementById(dynamic_fields).addEventListener("input",function(event) {
            checkSimilarity();
        });
    }
    getWebJSON("MPs.json",function (mpList) {
        makePoliticianList("PoliticianAskList",mpList,function (mp) {addMPToList(mp,"AddMPAskList",addMPsAskList); checkSimilarity();});
        makePoliticianList("PoliticianAnswerList",mpList,function (mp) {addMPToList(mp,"AddMPAnswerList",addMPsAnswerList); checkSimilarity();});
    },failure);
    getWebJSON("committees.json",function (committeeList) {
        makeCommitteeList("CommitteeAskList",committeeList,function (committee) {addCommitteeToList(committee,"AddCommitteeAskList",addCommitteesAskList); checkSimilarity();});
    },failure);

    updateAllList();
    checkSimilarity();
}