"use strict";

// Make a div into a list of links for politicians, from the MPs.json list politicianList, with given callback.

function makePoliticianList(elementIO,politicianList,callback) {
    let lastChamber = null;
    const div = add(document.getElementById(elementIO),"details");
    add(div,"summary").innerText="Choose Politician";
    for (const p of politicianList.mps) {
        if (lastChamber!==p.electorate.chamber) {
            add(div,"h6").innerText=p.electorate.chamber;
            lastChamber=p.electorate.chamber;
        } else div.append(" ");
        const a = add(div,"a");
        a.innerText = p.first_name+" "+p.surname;
        a.href = "#";
        a.onclick = function() { callback(p); return false; }
    }
}

// Make a div into a list of links for committees, from the committees.json list committeeList, with given callback.

function makeCommitteeList(elementIO,committeeList,callback) {
    let lastChamber = null;
    const div = add(document.getElementById(elementIO),"details");
    add(div,"summary").innerText="Choose Committee";
    for (const p of committeeList) {
        if (lastChamber!==p.jurisdiction) {
            add(div,"h6").innerText=p.jurisdiction;
            lastChamber=p.jurisdiction;
        } else div.append(" ");
        const a = add(div,"a");
        a.innerText = p.name;
        a.href = "#";
        a.onclick = function() { callback(p); return false; }
    }
}

// a useful callback helper for makePoliticianList that adds the politician to a list, and stores it in a list.
function addMPToList(mp,ui,list) {
    const mp_id = mp_id_of_mp(mp)
    const span = add(document.getElementById(ui),"span");
    const a = add(span,"a");
    a.innerText = "✖";
    a.href = "#";
    a.onclick = function () {
        span.remove();
        let e = list.findIndex(e=>e.MP===mp_id);
        if (e!== -1) { list[e]=list[e.length-1]; list.pop(); }
    }
    span.append(" "+mp_id_tostring(mp));
    list.push({"MP":mp_id});
}
function addCommitteeToList(committee,ui,list) {
    const span = document.getElementById(ui);
    span.append(" "+committee_id_tostring(committee));
    list.push({"Committee":committee_id_of_committee(committee)});
}


function mp_id_tostring(mp) { return mp.first_name+" "+mp.surname+" ("+mp.electorate.chamber+(mp.electorate.region?(" "+mp.electorate.region):"")+")"; }
function mp_id_of_mp(mp) { return {first_name : mp.first_name, surname: mp.surname, electorate : mp.electorate }; }
function committee_id_tostring(committee) { return committee.name+" ("+committee.jurisdiction+")"; }
function committee_id_of_committee(committee) { return {name : committee.name,jurisdiction : committee.jurisdiction }; }