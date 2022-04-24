import 'datatables.net-bs4';
import 'datatables.net-responsive-bs4';
import Chart from 'chart.js/auto';
import 'chartjs-adapter-date-fns';

function build_pil(job){
  return `<a class="nav-link ${job.idx == 0 ? 'active': ''}" data-toggle="pill" href="#assigned-agent-${job.idx}" role="tab" aria-controls="assigned-agent-${job.idx}" aria-selected="true">${job.idx}</a>`;
}

function build_aa_info(job){
  return `
<div class="tab-pane fade ${job.idx == 0 ? 'active' : ''} show" id="assigned-agent-${job.idx}" role="tabpanel" aria-labelledby="assigned-agent-${job.idx}">
  <div class="row">
    <div class="col-12 col-sm-5">
      <div class="info-box bg-light">
        <div class="info-box-content">
          <span class="info-box-text text-center text-muted"><i class="fas fa-fingerprint p-2 align-middle"></i>Agent Guid</span>
          <a href="/agents"><span class="info-box-number text-center text-muted mb-0">${job.agent_guid}</span></a>
        </div>
      </div>
    </div>
    <!-- /.col -->
    <div class="col-12 col-sm-3">
      <div class="info-box bg-light">
        <div class="info-box-content">
          <span class="info-box-text text-center text-muted"><i class="far fa-lightbulb"></i> Status</span>
          <span class="info-box-number text-center text-muted mb-0">${job.status}</span>
        </div>
      </div>
    </div>
    <!-- /.col -->
    <div class="col-6 col-sm-2">
      <div class="info-box bg-light">
        <div class="info-box-content">
          <span class="info-box-text text-center text-muted"><i class="fas fa-microchip"></i> CPUs</span>
          <span class="info-box-number text-center text-muted mb-0">${job.cpus}</span>
        </div>
      </div>
    </div>
    <!-- /.col -->
    <div class="col-6 col-sm-2">
      <div class="info-box bg-light">
        <div class="info-box-content">
          <span class="info-box-text text-center text-muted"><i class="fas fa-memory"></i> RAM</span>
          <span class="info-box-number text-center text-muted mb-0">${job.ram}</span>
        </div>
      </div>
    </div>
    <!-- /.col -->
  </div>
  <!-- /.row -->
  <div class="row">
    <div class="col-12">
      <div>
        <h4>Agent last message</h4>
        <pre>${job.last_msg || 'No message received'}</pre>
      </div>
      <div>
        <h4>Container logs</h4>
        <pre>${job.log || "No log received"}</pre>
      </div>
    </div>
  </div>
</div>`;
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
        "render": renderAnalyzeStatus
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

    var aa_navs = $("#assigned-agents-navs")
    var aa_tabs = $("#assigned-agents-tabs")

    job.jobs.forEach(job => {
      var nav = $(build_pil(job));
      nav.appendTo(aa_navs);

      var tab = $(build_aa_info(job));
      tab.appendTo(aa_tabs);
    });
  } catch (err) {
    iziToast.error({
      title: err.message,
      message: err.stack
    });
  }
}

async function fetch_stats(data, guid){
  let response = await fetch(
    `/api/stats/${guid}`,
    {
      method: "POST",
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(data),
    }
  );
  return response.json();
}

function build_chart(ctx, datasets) {
  var myChart = new Chart(ctx, {
    type: 'line',
    data: datasets,
    options: {
      scales: {
        x: {
          type: 'time',
          ticks: {
              autoSkip: true,
              maxTicksLimit: 10
          }
        }
      },
    }
  });
}

async function init_execs_graph(guid){
  try {
    let res = await fetch_stats({query: 'execs_per_sec'}, guid);
    let data = res.data.result;
    let datasets = parsePromData(data, "guid");

    let ctx = $("#execs-graph");
    build_chart(ctx, datasets);
  } catch(err) {
    return;
  }
}

async function init_crashes_graph(guid){
  try {
    let res = await fetch_stats({query: 'saved_crashes'}, guid);
    let data = res.data.result;
    let datasets = parsePromData(data, "guid");

    let ctx = $("#crashes-graph");
    build_chart(ctx, datasets);
  } catch(err) {
    return;
  }
}

async function init_edges_graph(guid){
  try {
    let res = await fetch_stats({query: 'edges_found'}, guid);
    let data = res.data.result;
    let datasets = parsePromData(data, "guid");

    let ctx = $("#edges-graph");
    build_chart(ctx, datasets);
  } catch(err) {
    return;
  }
}

async function init_cycle_graph(guid){
  try {
    let res = await fetch_stats({query: 'cycle_done'}, guid);
    let data = res.data.result;
    let datasets = parsePromData(data, "guid");

    let ctx = $("#cycle-graph");
    build_chart(ctx, datasets);
  } catch(err) {
    return;
  }
}

async function main(){
  var guid = window.location.pathname.split("/").pop();
  init_crash_table(guid);
  await Promise.all([
    init_job_info(guid),
    init_execs_graph(guid),
    init_crashes_graph(guid),
    init_edges_graph(guid),
    init_cycle_graph(guid)
  ]);
}

(async() => {
  await main()
})();
