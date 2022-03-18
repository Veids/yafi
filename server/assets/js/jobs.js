import $ from 'jquery';
window.jQuery = $;
window.$ = $;

import 'datatables.net-bs4';
import 'datatables.net-responsive-bs4';

function renderBadge(data, type) {
  if (type === "display"){
    return `<span class="badge badge-secondary">${data}</span>`;
  }
  return data;
}

function renderAgentType(data, type) {
  if (type === "display"){
    var icon = "";
    switch (data) {
      case "linux":
        icon = "fab fa-linux";
        break;
      case "windows":
        icon = "fab fa-windows";
        break;
    }
    return `<i class="${icon}"></i> ${data}`;
  }
  return data;
}

function renderJobStatus(data, type) {
  if (type === "display") {
    var badge = "badge-secondary";

    switch (data){
      case "init":
      case "alive":
        badge = "badge-primary";
        break;
      case "completed":
        badge = "badge-success";
        break;
      case "error":
        badge = "badge-danger";
        break;
    }
    return `<span class="badge ${badge}">${data}</span>`;
  }
  return data;
}

function main(){
  var t = $("#jobs-table").DataTable({
    "responsive": true,
    "autoWidth": false,
    "ajax": {
      "url": "api/jobs",
      "dataSrc": ""
    },
    "columns": [
      { "data": "guid" },
      {
        "data": "name",
        "render": $.fn.dataTable.render.text()
      },
      {
       "data": "description",
        "render": $.fn.dataTable.render.text()
      },
      { "data": "creation_date" },
      { 
        "data": "agent_type",
        "className": "text-center",
        "render": renderAgentType
      },
      { 
        "data": "cpus",
        "className": "text-center",
        "render": renderBadge
      },
      { 
        "data": "ram",
        "className": "text-center",
        "render": renderBadge
      },
      { 
        "data": "timeout",
        "className": "text-center",
        "render": $.fn.dataTable.render.text()
      },
      { 
        "data": "status",
        "className": "text-center",
        "render": renderJobStatus
      },
    ],
    "order": [[3, "desc"]]
  });

  $(t.table().container()).on("click", "tbody tr", function(){
    var row = t.row(this);
    window.location = "job/" + row.data().guid;
  });
}

$(main);
