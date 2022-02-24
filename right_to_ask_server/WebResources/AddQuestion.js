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

// function taken from tweetnacl-util, by Dmitry Chestnykh and Devi Mandiri, public domain.
function decodeBase64(s) {
    var i, d = atob(s), b = new Uint8Array(d.length);
    for (i = 0; i < d.length; i++) b[i] = d.charCodeAt(i);
    return b;
}

// function taken from tweetnacl-util, by Dmitry Chestnykh and Devi Mandiri, public domain.
function encodeBase64(arr) {
    var i, s = [], len = arr.length;
    for (i = 0; i < len; i++) s.push(String.fromCharCode(arr[i]));
    return btoa(s.join(''));
}

function check_signature(signed) {
    const message = signed.message;
    const signature = signed.signature;
    function success(publicKey) {
        const messageUint8Array = (new TextEncoder()).encode(message);
        const signatureUint8Array = decodeBase64(signature);
        const publicKeyUint8Array = decodeBase64(publicKey);
        let verified = nacl.sign.detached.verify(messageUint8Array,signatureUint8Array,publicKeyUint8Array);
        status("Verified "+verified+" for "+signature+" as a signature for "+message+" public key "+publicKey);
        if (crypto && crypto.subtle) {
            // actually can't do anything useful
        }
    }
    getWebJSON("get_server_public_key_raw",success,failure)
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
    let message = JSON.stringify(command);
    let user = document.getElementById("UID").value;
    let privateKey = document.getElementById("PrivateKey").value;
    const privateKeyUint8Array = decodeBase64(privateKey);
    const messageUint8Array = (new TextEncoder()).encode(message);
    const signatureUint8Array = nacl.sign.detached(messageUint8Array,privateKeyUint8Array);
    console.log(signatureUint8Array);
    let signature = encodeBase64(signatureUint8Array);
    let signed_message = { message:message, user:user, signature:signature };
    getWebJSON("new_question",success,failure,JSON.stringify(signed_message),"application/json")
}

window.onload = function () {
    document.getElementById("Add").onclick = addQuestion;
    updateQuestionList();
}