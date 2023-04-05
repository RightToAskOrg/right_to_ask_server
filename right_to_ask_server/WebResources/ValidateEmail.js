"use strict";

function getWhy() {
    switch (document.getElementById("Purpose").value) {
        case "AccountValidation" : return "AccountValidation";
        case "MP" : return {"AsMP":true};
        case "MPStaffer" : return  {"AsMP":false};
        case "Org" : return "AsOrg";
        case "revokeMP" : return {"RevokeMP" : [document.getElementById("Revokee").value,true]};
        case "revokeMPStaffer" : return {"RevokeMP" : [document.getElementById("Revokee").value,false]};
        case "revokeOrg" : return {"RevokeOrg" : document.getElementById("Revokee").value};
    }
    failure("Need to say why");
    return null;
}

let email_id = null;

function reportAcceptedProof(signature) {
    if (signature) {
        check_signature(signature);
        status("The server believes you! Bulletin board "+signature.message);
    } else {
        status("The server believes you but didn't put anything on the bulletin board.")
    }
}
function RequestEmail() {
    function success(result) {
        console.log(result);
        if (result.Ok) {
            if (result.Ok.EmailSent) {
                email_id = result.Ok.EmailSent;
                status("Requested email successfully. Email id "+email_id);
            } else if (result.Ok.hasOwnProperty("AlreadyValidated")) {
                reportAcceptedProof(result.Ok.AlreadyValidated);
            } else {
                status("I Don't understand the server")
            }
        } else {
            status("Tried request email. Got Error message "+result.Err);
        }
    }
    const command = {
        name : document.getElementById("BadgeName").value,
        why : getWhy()
    }
    let signed_message = sign_message(command);
    signed_message.email =  document.getElementById("Email").value;
    getWebJSON("request_email_validation",success,failure,JSON.stringify(signed_message),"application/json")
}

function VerifyCode() {
    function success(result) {
        console.log(result);
        if (result.hasOwnProperty("Ok")) {
            reportAcceptedProof(result.Ok);
        } else if (result.Err) {
            status("Tried request email. Got Error message "+result.Err);
        } else {
            status("Code seems to have verified successfully. No Bulletin board entry.")
        }
    }
    let command = {
        hash : email_id,
        code : parseInt(document.getElementById("Code").value,10)
    }
    let signed_message = sign_message(command);
    getWebJSON("email_proof",success,failure,JSON.stringify(signed_message),"application/json")
}

window.onload = function () {
    document.getElementById("RequestEmail").onclick = RequestEmail;
    document.getElementById("VerifyCode").onclick = VerifyCode;
    getWebJSON("MPs.json",function (mpList) {
        makePoliticianList("ChooseMP",mpList,function (mp) {
            document.getElementById("Email").value=mp.email;
            document.getElementById("BadgeName").value=mp.first_name+" "+mp.surname+" "+mp.email.slice(mp.email.indexOf("@"));
        });
    },failure);
}