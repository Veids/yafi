import Chart from 'chart.js/auto';
import 'chartjs-adapter-date-fns';

async function init_agent_stats(){
  const response = await fetch("/api/agents");
  const agents = await response.json();

  $("#agents_total h3").text(agents.length);
  $("#agents_total .overlay").remove();

  var alive_agents = agents.filter(agent => agent.status == "up");
  $("#agents_alive h3").text(alive_agents.length);
  $("#agents_alive .overlay").remove();
}

async function init_job_stats(){
  const response = await fetch("/api/job");
  const stats = await response.json();

  $("#jobs_alive h3").text(stats.alive);
  $("#jobs_alive .overlay").remove();

  $("#jobs_completed h3").text(stats.completed);
  $("#jobs_completed .overlay").remove();

  $("#jobs_error h3").text(stats.error);
  $("#jobs_error .overlay").remove();
}

async function init_crash_stats(){
  const response = await fetch("/api/crash");
  const stats = await response.json();

  $("#crashes_total h3").text(stats.total);
  $("#crashes_total .overlay").remove();
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

async function fetch_stats(data){
  let response = await fetch(
    '/api/stats',
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

async function init_crashes_graph(){
  try {
    let res = await fetch_stats({query: 'sum by (guid) (fuzzing{type="saved_crashes"})'});
    let data = res.data.result;
    let datasets = parsePromData(data, "guid");

    let ctx = $("#crashes-graph");
    build_chart(ctx, datasets);
  } catch(err) {
    return;
  }
}

async function init_edges_graph(){
  try {
    let res = await fetch_stats({query: 'fuzzing{type="edges_found",banner="0"}'});
    let data = res.data.result;
    let datasets = parsePromData(data, "guid");

    let ctx = $("#edges-graph");
    build_chart(ctx, datasets);
  } catch(err) {
    return;
  }
}

async function init_execs_graph(){
  try {
    let res = await fetch_stats({query: 'sum by (guid) (fuzzing{type="execs_per_sec"})'});
    let data = res.data.result;
    let datasets = parsePromData(data, "guid");

    let ctx = $("#execs-graph");
    build_chart(ctx, datasets);
  } catch(err) {
    return;
  }
}

async function init_hangs_graph(){
  try {
    let res = await fetch_stats({query: 'sum by (guid) (fuzzing{type="saved_hangs"})'});
    let data = res.data.result;
    let datasets = parsePromData(data, "guid");

    let ctx = $("#hangs-graph");
    build_chart(ctx, datasets);
  } catch(err) {
    return;
  }
}

async function init_stats(){
  await Promise.all([
    init_agent_stats(),
    init_job_stats(),
    init_crash_stats(),
    init_crashes_graph(),
    init_edges_graph(),
    init_execs_graph(),
    init_hangs_graph()
  ]);
}

(async() => {
  await init_stats()
})();
