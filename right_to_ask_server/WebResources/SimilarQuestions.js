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

let currently_pending_check_similarity = false;
let should_do_new_check_similarity = false;
function checkSimilarity() {
    if (currently_pending_check_similarity) { should_do_new_check_similarity=true; return; }
    function pendingCheck() {
        currently_pending_check_similarity=false;
        if (should_do_new_check_similarity) {
            should_do_new_check_similarity=false;
            checkSimilarity();
        }
    }
    function success(data) {
        if (data.Err) failure(data.Err);
        else {
            console.log(data.Ok);
            const div = document.getElementById("SimilarQuestions");
            removeAllChildElements(div);
            for (const possibility of data.Ok) {
                let line = add(div,"div","SimilarQuestionLine");
                add(line,"span","score").innerText = possibility.score.toFixed(2);
                function foundQuestion(data) {
                    if (data.Ok) line.append(" ["+data.Ok.author+"] "+data.Ok.question_text);
                }
                getWebJSON(getURL("get_question",{question_id:possibility.id}),foundQuestion,failure);
            }
            pendingCheck();
        }
    }
    function failurePending(message) {
        failure(message);
        pendingCheck();
    }
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
    getWebJSON("similar_questions",success,failurePending,JSON.stringify(command),"application/json");
}

let addMPsAskList = [];
let addCommitteesAskList = [];
let addMPsAnswerList = [];

window.onload = function () {
    document.getElementById("entry").addEventListener("input",function(event) {
        checkSimilarity();
    });
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