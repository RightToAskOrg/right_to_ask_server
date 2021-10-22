"use strict";

/** Add a line to the status display.
 * @param line{string} line to add to the status */
function status(line) {
    add(document.getElementById("status"),"div").innerText=line;
}
function failure(error) {
    status("Error : "+error);
}





function addLink(where,hashvalue) {
    const link = add(where,"a");
    link.innerText=hashvalue;
    link.href = "LookupHash.html?hash="+hashvalue;
}

/** Add a text node
 * @param where{HTMLElement}
 * @param label{string}
 */
function addText(where,label) {
    where.appendChild(document.createTextNode(label));
}

function addLabeledLink(where,label,hashvalue) {
    addText(where,label);
    addLink(where,hashvalue);
}