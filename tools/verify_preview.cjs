// Real-browser smoke check using already installed Chrome; no dependencies.
const fs = require('node:fs');
const path = require('node:path');
const {spawn} = require('node:child_process');
const delay = ms => new Promise(resolve => setTimeout(resolve,ms));
const parent = process.env.ARCENAL_RUNTIME_PARENT;
function check(condition, message) { if (!condition) throw new Error(message); }
let browser, directory, socket;
async function main() {
  if (!parent || !fs.statSync(parent).isDirectory()) throw new Error('Existing runtime parent required');
  directory = fs.mkdtempSync(path.join(parent,'arcenal-check-'));
  browser = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',[
    '--headless=new','--disable-gpu','--remote-debugging-port=0','--remote-debugging-address=127.0.0.1',
    '--user-data-dir='+directory,'--no-first-run','--no-default-browser-check','about:blank'
  ],{stdio:'ignore'});
  const portFile=path.join(directory,'DevToolsActivePort');
  for(let n=0;n<100&&!fs.existsSync(portFile);n++) await delay(100);
  if(!fs.existsSync(portFile)) throw new Error('Chrome unavailable');
  const port=fs.readFileSync(portFile,'utf8').split('\n')[0];
  const targets=await(await fetch('http://127.0.0.1:'+port+'/json/list')).json();
  socket=new WebSocket(targets.find(t=>t.type==='page').webSocketDebuggerUrl);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  let next=0;const pending=new Map();
  socket.addEventListener('message',event=>{
    const message=JSON.parse(event.data);const item=pending.get(message.id);if(!item)return;
    pending.delete(message.id);clearTimeout(item.timer);
    if(message.error)item.reject(new Error(message.error.message));else item.resolve(message.result);
  });
  function command(method,params={}){return new Promise((resolve,reject)=>{const id=++next;const timer=setTimeout(()=>{pending.delete(id);reject(new Error('CDP timeout'));},45000);pending.set(id,{resolve,reject,timer});socket.send(JSON.stringify({id,method,params}));});}
  async function evaluate(expression){const result=await command('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(result.exceptionDetails)throw new Error('Browser evaluation failed');return result.result.value;}
  await command('Page.enable');
  await command('Emulation.setDeviceMetricsOverride',{width:1440,height:1000,deviceScaleFactor:1,mobile:false});
  await command('Page.navigate',{url:'http://127.0.0.1:8080/agents/elise'});await delay(1500);
  for(let n=0;n<180;n++){if(await evaluate(`document.getElementById('recruitment-data')?.dataset.loaded==='true'`))break;await delay(250);}
  const recruitment=await evaluate(`({loaded:document.getElementById('recruitment-data')?.dataset.loaded,tabs:[...document.querySelectorAll('[data-recruitment-tab]')].map(e=>e.textContent),columns:document.querySelectorAll('.pipeline-column').length,cards:document.querySelectorAll('#candidate-pipeline .candidate-card').length})`);
  check(recruitment.loaded==='true'&&recruitment.columns===6&&recruitment.tabs.length===6,'Recruitment pipeline failed');
  console.log('Recrutement:',JSON.stringify(recruitment));
  for(const [view,target] of [['pool','pool-results'],['jobs','job-results'],['settings','recruitment-connectors'],['activity','recruitment-activity']]){
    await evaluate(`document.querySelector('[data-recruitment-tab="${view}"]').click()`);
    const result=await evaluate(`({view:'${view}',visible:!document.querySelector('[data-recruitment-view="${view}"]').hidden,items:document.getElementById('${target}').children.length,overflow:document.documentElement.scrollWidth>innerWidth})`);
    check(result.visible&&result.items>0&&!result.overflow,'Recruitment screen failed: '+view);console.log('Onglet:',JSON.stringify(result));
  }
  await evaluate(`document.querySelector('[data-recruitment-tab="candidates"]').click();document.getElementById('candidate-search').value='zzzzxxxx-no-match';document.getElementById('candidate-search').dispatchEvent(new Event('input'));`);
  check(await evaluate(`document.querySelectorAll('#candidate-pipeline .candidate-card').length===0`),'Candidate search failed');
  await evaluate(`document.getElementById('candidate-search').value='';document.getElementById('candidate-search').dispatchEvent(new Event('input'));document.querySelector('#candidate-pipeline .candidate-card')?.click()`);
  check(await evaluate(`document.getElementById('recruitment-detail').open`),'Candidate details failed');
  await command('Input.dispatchKeyEvent',{type:'keyDown',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});
  await command('Input.dispatchKeyEvent',{type:'keyUp',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});
  check(await evaluate(`!document.getElementById('recruitment-detail').open`),'Dialog keyboard close failed');
  await command('Emulation.setDeviceMetricsOverride',{width:360,height:800,deviceScaleFactor:1,mobile:true});
  for(const view of ['candidates','pool','jobs','settings','activity']){
    await evaluate(`document.querySelector('[data-recruitment-tab="${view}"]').click()`);
    check(await evaluate(`document.documentElement.scrollWidth<=innerWidth`),'Mobile recruitment overflow: '+view);
  }
  console.log('Cinq onglets mobiles: aucun débordement');
  await command('Emulation.setDeviceMetricsOverride',{width:1440,height:1000,deviceScaleFactor:1,mobile:false});
  await evaluate(`document.querySelector('[data-recruitment-tab="conversation"]').click()`);
  const desktop=await evaluate(`({title:document.title,chat:!!document.getElementById('chat-form'),overflow:document.documentElement.scrollWidth>innerWidth,remoteTokenInBrowser:typeof window.__HERMES_SESSION_TOKEN__!=='undefined'})`);
  check(desktop.chat&&!desktop.overflow&&!desktop.remoteTokenInBrowser,'Desktop/privacy check failed');
  console.log('Desktop:',JSON.stringify(desktop));
  await evaluate(`document.getElementById('suggestion').click()`);
  await command('Input.dispatchKeyEvent',{type:'keyDown',key:'Tab',code:'Tab',windowsVirtualKeyCode:9});
  await command('Input.dispatchKeyEvent',{type:'keyUp',key:'Tab',code:'Tab',windowsVirtualKeyCode:9});
  const keyboard=await evaluate(`({submitReachable:document.activeElement===document.querySelector('#chat-form button')})`);
  check(keyboard.submitReachable,'Keyboard check failed');console.log('Clavier:',JSON.stringify(keyboard));
  await command('Emulation.setDeviceMetricsOverride',{width:360,height:800,deviceScaleFactor:1,mobile:true});await delay(200);
  const mobile=await evaluate(`({width:innerWidth,overflow:document.documentElement.scrollWidth>innerWidth})`);
  check(!mobile.overflow,'Mobile overflow');console.log('Mobile:',JSON.stringify(mobile));
  await evaluate(`document.getElementById('theme').value='dark';document.getElementById('theme').dispatchEvent(new Event('change'));`);
  const dark=await evaluate(`({background:getComputedStyle(document.body).backgroundColor})`);
  check(dark.background==='rgb(23, 25, 30)','Gratitude dark palette mismatch');console.log('Thème sombre:',JSON.stringify(dark));
  await evaluate(`document.getElementById('theme').value='light';document.getElementById('theme').dispatchEvent(new Event('change'));`);
  const light=await evaluate(`({background:getComputedStyle(document.body).backgroundColor})`);
  check(light.background==='rgb(255, 249, 239)','Gratitude light palette mismatch');console.log('Thème clair:',JSON.stringify(light));
  await evaluate(`document.getElementById('hermes-test').click()`);
  for(let n=0;n<120;n++){await delay(250);if(!(await evaluate(`document.getElementById('hermes-test').disabled`)))break;}
  const identity=await evaluate(`document.getElementById('hermes-status').textContent`);
  check(identity.includes('Hermes joignable')&&identity.includes('auteur'),'SSO/Hermes status failed');console.log('Identité/connexion:',identity);
  if(process.env.ARCENAL_CHECK_CHAT==='1'){
    await evaluate(`document.getElementById('chat-message').value='Présente-toi en deux phrases pour ce test de connexion, sans outil ni action externe.';document.getElementById('chat-form').dispatchEvent(new Event('submit',{bubbles:true,cancelable:true}));`);
    for(let n=0;n<145;n++){await delay(1000);if(!(await evaluate(`document.querySelector('#chat-form button').disabled`)))break;}
    const dialogue=await evaluate(`({status:document.getElementById('chat-status').textContent,reply:document.querySelector('.bubble.assistant:last-of-type')?.innerText.slice(0,1200)||'',error:document.querySelector('.bubble.error')?.innerText||'',messageRetained:document.getElementById('chat-message').value.length>0})`);
    console.log('Dialogue réel:',JSON.stringify(dialogue));
    if(!dialogue.status.includes('Réponse reçue de Hermes')) process.exitCode=2;
  }
}
main().catch(error=>{console.error(error.message);process.exitCode=1;}).finally(async()=>{
  if(socket)socket.close();
  if(browser){browser.kill('SIGTERM');for(let n=0;n<30&&browser.exitCode===null;n++)await delay(100);if(browser.exitCode===null){browser.kill('SIGKILL');await delay(1000);}}
  if(directory)fs.rmSync(directory,{recursive:true,force:true,maxRetries:5,retryDelay:200});
});
