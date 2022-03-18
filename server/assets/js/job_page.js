import $ from 'jquery';
window.jQuery = $;
window.$ = $;

import 'datatables.net-bs4';
import 'datatables.net-responsive-bs4';

function build_job_row(job){
  return `
    <tr data-widget="expandable-table" aria-expanded="false">
      <td class="text-center">${job.agent_guid}</td>
    </tr>
    <tr class="expandable-body d-none">
      <td>
        <div class="container">
          <div class="container">
            <strong>
              <i class="fas fa-fingerprint"></i> Collection GUID
            </strong>
            <p class="text-muted mb-0">${job.collection_guid}</p>
          </div>
          <hr class="my-2">
          <div class="container">
            <strong>
              <i class="fas fa-star"></i> Master
            </strong>
            <p class="text-muted mb-0">${job.master}</p>
          </div>
          <hr class="my-2">
          <div class="container row">
            <div class="col-md-6 border-right">
              <strong class="align-middle">
                <i class="fas fa-microchip"></i> CPUs
              </strong>
              <span class="agent-badge float-right">${job.cpus}</span>
            </div>
            <div class="col-md-6">
              <strong class="align-middle">
                <i class="fas fa-memory"></i> RAM
              </strong>
              <span class="agent-badge float-right">${job.ram}</span>
            </div>
          </div>
          <hr class="my-2">
          <div class="container">
            <strong>
              <i class="fas fa-sticky-note"></i> Last message
            </strong>
            <p class="text-muted mb-0">${job.last_msg}</p>
          </div>
          <hr class="my-2">
          <div class="container">
            <strong>
              <i class="far fa-lightbulb"></i> Status
            </strong>
            <p class="text-muted mb-0">${job.status}</p>
          </div>
        </div>
      </td>
    </tr>
  `;
}


async function job_stop(guid) {
  try {
    const response = await fetch(`/api/job/${guid}/stop`);
    iziToast.success({
      title: 'OK',
      message: 'Job stop request sent!',
    });
  } catch (err) {
    iziToast.error({
      title: 'Error',
      message: err.statusText,
    });
  }
}

function init_crash_table(guid){
  var t = $("#crash-table").DataTable({
    "responsive": true,
    "autoWidth": false,
    "ajax": {
      "url": `/api/job/${guid}/crashes`,
      "dataSrc": ""
    },
    "columns": [
      { "data": "guid" },
      {
        "data": "name",
        "render": $.fn.dataTable.render.text()
      },
      {
       "data": "analyzed",
        "render": $.fn.dataTable.render.text()
      },
    ],
  });

  $(t.table().container()).on("click", "tbody tr", function(){
    var row = t.row(this);
    window.location = "/crash/" + row.data().guid;
  });
}

async function init_job_info(guid){
  try {
    const response = await fetch(`/api/job/${guid}`);
    const job = await response.json();

    var card = $("#job-info");
    card.find("#name").text(job.job_collection.name);
    card.find("#description").text(job.job_collection.description);
    card.find("#guid").text(job.job_collection.guid);
    card.find("#created").text(job.job_collection.creation_date);
    card.find("#cpus").text(job.job_collection.cpus);
    card.find("#ram").text(job.job_collection.ram);
    card.find("#timeout").text(job.job_collection.timeout);
    card.find("#status").text(job.job_collection.status);
    if (job.job_collection.status == "alive" || job.job_collection.status == "init") {
      var stop = card.find("#stop");
      stop.click(async function(event){
        await job_stop(guid);
        event.preventDefault();
      });
      stop.show();
    }
    card.find(".overlay").remove();

    var assigned_agents = $("#assigned-agents tbody"); 
    job.jobs.forEach(job => {
      var tr = $(build_job_row(job));
      tr.appendTo(assigned_agents);
      tr.ExpandableTable("init");
    });
  } catch (err) {
    iziToast.error({
      title: err.message,
      message: err.stack
    });
  }
}

async function main(){
  var guid = window.location.pathname.split("/").pop();
  init_crash_table(guid);
  await init_job_info(guid);
}

(async() => {
  await main()  
})();
