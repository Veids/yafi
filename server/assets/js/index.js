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

async function init_stats(){
  await Promise.all([init_agent_stats(), init_job_stats(), init_crash_stats()]);
}

(async() => {
  await init_stats()  
})();
