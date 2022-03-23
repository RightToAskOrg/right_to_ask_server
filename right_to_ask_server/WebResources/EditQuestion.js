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
    let askList = addMPsAskList.slice(); // make a shallow copu
    let answerList = addMPsAnswerList.slice(); // make a shallow copu
    parseUsers(askList,"AddUserAskList","User");
    parseUsers(answerList,"AddUserAnswerList","User");
    parseUsers(askList,"AddDomainAskList","Organisation");
    parseUsers(answerList,"AddDomainAnswerList","Organisation");
    if (askList.length>0) { command.mp_who_should_ask_the_question = askList; }
    if (answerList.length>0) { command.entity_who_should_answer_the_question = answerList; }
    if (document.getElementById("PermissionsAsk").checked!==(question.who_should_ask_the_question_permissions==="WriterOnly")) {
        command.who_should_ask_the_question_permissions = document.getElementById("PermissionsAsk").checked?"WriterOnly":"Others";
    }
    if (document.getElementById("PermissionsAnswer").checked!==(question.who_should_answer_the_question_permissions==="WriterOnly")) {
        command.who_should_answer_the_question_permissions = document.getElementById("PermissionsAnswer").checked?"WriterOnly":"Others";
    }
    let signed_message = sign_message(command);
    getWebJSON("edit_question",success,failure,JSON.stringify(signed_message),"application/json")
}

function makePersonListDescription(desc) {
    let res = "";
    if (desc) for (const p of desc) {
        res+=JSON.stringify(p);
        res+=" ";
    }
    return res;
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
        document.getElementById("AddMPAskList").innerText="";
        document.getElementById("AddMPAnswerList").innerText="";
        document.getElementById("AddUserAskList").value="";
        document.getElementById("AddUserAnswerList").value="";
        document.getElementById("AddDomainAskList").value="";
        document.getElementById("AddDomainAnswerList").value="";
        document.getElementById("CurrentAskList").innerText=makePersonListDescription(question.mp_who_should_ask_the_question);
        document.getElementById("CurrentAnswerList").innerText=makePersonListDescription(question.entity_who_should_answer_the_question);
        addMPsAskList = [];
        addMPsAnswerList = [];
    } else if (questionInfo.Err) failure("Error : "+questionInfo.Err);
}

function updateQuestion() {
    getWebJSON(getURL("get_question",{question_id:question_id}),setQuestion,failure);
}

let addMPsAskList = [];
let addMPsAnswerList = [];
function addMPToList(mp,ui,list) {
    const span = document.getElementById(ui);
    span.append(" "+mp.first_name+" "+mp.surname+" ("+mp.electorate.chamber+(mp.electorate.region?(" "+mp.electorate.region):"")+")");
    list.push({"MP":{first_name : mp.first_name, surname: mp.surname, electorate : mp.electorate }});
}

window.onload = function () {
    question_id = new URLSearchParams(window.location.search).get("question_id");
    document.getElementById("Edit").onclick = editQuestion;
    updateQuestion();
    getWebJSON("MPs.json",function (mpList) {
        makePoliticianList("PoliticianAskList",mpList,function (mp) {addMPToList(mp,"AddMPAskList",addMPsAskList)});
        makePoliticianList("PoliticianAnswerList",mpList,function (mp) {addMPToList(mp,"AddMPAnswerList",addMPsAnswerList)});
    },failure);
}