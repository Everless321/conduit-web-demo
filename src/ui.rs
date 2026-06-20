//! The entire management UI, embedded as one self-contained HTML page so the
//! binary needs no external assets. A small vanilla-JS SPA: login screen, then
//! a sidebar shell with Dashboard / Servers / Tokens views. Bilingual (EN/中文).
//! Talks to `/api/*`; the admin password is held in `localStorage` and sent as
//! a bearer token.

pub const INDEX_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Conduit Console</title>
<style>
  :root{
    --bg:#f5f5f7; --bg2:#ffffff; --panel:#ffffff; --panel2:#f0f0f2;
    --line:rgba(0,0,0,.09); --line2:rgba(0,0,0,.14);
    --fg:#1d1d1f; --mut:#6e6e73; --dim:#8e8e93;
    --acc:#0071e3; --acc2:#0064d2; --accbg:rgba(0,113,227,.10);
    --ok:#1e9e54; --okbg:rgba(52,199,89,.14); --bad:#d70015; --badbg:rgba(255,59,48,.12);
    --warn:#b25000;
    --r:8px; --r2:12px;
    --shadow:0 12px 40px rgba(0,0,0,.16);
    --shadow-sm:0 1px 2px rgba(0,0,0,.05);
    --font:-apple-system,BlinkMacSystemFont,'SF Pro Text','SF Pro Display','Helvetica Neue',system-ui,sans-serif;
    --mono:ui-monospace,'SF Mono',Menlo,Consolas,monospace;
  }
  *{box-sizing:border-box;margin:0;padding:0}
  body{background:var(--bg);color:var(--fg);font-family:var(--font);font-size:14px;line-height:1.5;-webkit-font-smoothing:antialiased}
  a{color:var(--acc);text-decoration:none}
  button{font-family:inherit;cursor:pointer;border:0;background:none;color:inherit}
  svg{width:18px;height:18px;stroke:currentColor;fill:none;stroke-width:1.7;stroke-linecap:round;stroke-linejoin:round;flex:none}
  input,select,textarea{width:100%;background:var(--bg2);border:1px solid var(--line2);color:var(--fg);border-radius:7px;padding:9px 11px;font:inherit;transition:border-color .15s,box-shadow .15s}
  input:focus,select:focus,textarea:focus{outline:none;border-color:var(--acc);box-shadow:0 0 0 3.5px rgba(0,113,227,.22)}
  textarea{min-height:84px;font-family:var(--mono);font-size:12.5px;resize:vertical}
  select{appearance:none;background-image:url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%238e8e93' stroke-width='2.2'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");background-repeat:no-repeat;background-position:right 10px center;padding-right:34px}
  label{display:block;font-size:12px;color:var(--mut);margin-bottom:6px;font-weight:500}
  .btn{display:inline-flex;align-items:center;justify-content:center;gap:7px;padding:8px 15px;border-radius:7px;font-weight:500;font-size:13.5px;background:var(--acc);color:#fff;transition:background .15s,transform .05s;white-space:nowrap}
  .btn:hover{background:var(--acc2)} .btn:active{transform:scale(.985)}
  .btn.ghost{background:var(--bg2);color:var(--fg);border:1px solid var(--line2);box-shadow:var(--shadow-sm)}
  .btn.ghost:hover{background:var(--panel2)}
  .btn.danger{background:var(--bg2);color:var(--bad);border:1px solid var(--line2);box-shadow:var(--shadow-sm);padding:6px 12px;font-size:12.5px}
  .btn.danger:hover{background:var(--badbg);border-color:rgba(215,0,21,.3)}
  .btn.sm{padding:6px 12px;font-size:12.5px}
  .hidden{display:none!important}

  /* login */
  #login{min-height:100vh;display:flex;align-items:center;justify-content:center;padding:24px;background:linear-gradient(180deg,#fbfbfd,#ececed)}
  .login-card{width:100%;max-width:380px;background:var(--panel);border:1px solid var(--line);border-radius:18px;padding:34px 30px;box-shadow:0 20px 60px rgba(0,0,0,.12),0 1px 0 rgba(255,255,255,.6) inset;text-align:center}
  .login-card .logo{display:flex;flex-direction:column;align-items:center;gap:12px;margin-bottom:4px}
  .login-card .logo .mark{width:56px;height:56px;border-radius:14px;background:linear-gradient(160deg,#3b9bff,#0071e3);display:flex;align-items:center;justify-content:center;color:#fff;box-shadow:0 6px 16px rgba(0,113,227,.32)}
  .login-card .logo .mark svg{width:30px;height:30px;stroke-width:2}
  .login-card h1{font-size:22px;font-weight:600;letter-spacing:-.02em}
  .login-card .sub{color:var(--mut);font-size:13px;margin-bottom:24px}
  .login-card form{display:flex;flex-direction:column;gap:12px;text-align:left}
  .login-card .btn{padding:10px}

  /* shell */
  #app{display:grid;grid-template-columns:230px 1fr;min-height:100vh}
  aside{background:rgba(246,246,248,.8);backdrop-filter:saturate(180%) blur(20px);border-right:1px solid var(--line);display:flex;flex-direction:column;padding:18px 12px}
  aside .brand{display:flex;align-items:center;gap:10px;padding:6px 8px 20px}
  aside .brand .mark{width:32px;height:32px;border-radius:8px;background:linear-gradient(160deg,#3b9bff,#0071e3);display:flex;align-items:center;justify-content:center;color:#fff;box-shadow:0 2px 6px rgba(0,113,227,.3)}
  aside .brand .mark svg{width:18px;height:18px;stroke-width:2}
  aside .brand .name{font-weight:600;font-size:15px;letter-spacing:-.01em}
  aside .brand .tag{font-size:11px;color:var(--dim)}
  nav{display:flex;flex-direction:column;gap:2px;flex:1}
  nav .navitem{display:flex;align-items:center;gap:10px;padding:8px 11px;border-radius:7px;color:var(--fg);font-weight:450;font-size:13.5px;transition:background .12s,color .12s;width:100%;text-align:left}
  nav .navitem svg{color:var(--mut);width:17px;height:17px}
  nav .navitem:hover{background:rgba(0,0,0,.05)}
  nav .navitem.active{background:var(--acc);color:#fff}
  nav .navitem.active svg{color:#fff}
  aside .foot{display:flex;flex-direction:column;gap:10px;border-top:1px solid var(--line);padding-top:14px}
  .langtoggle{display:flex;background:rgba(120,120,128,.12);border-radius:8px;padding:2px;gap:2px}
  .langtoggle button{flex:1;padding:5px;border-radius:6px;font-size:12px;font-weight:500;color:var(--fg);transition:.12s}
  .langtoggle button.on{background:var(--bg2);color:var(--fg);box-shadow:0 1px 3px rgba(0,0,0,.14)}

  main{display:flex;flex-direction:column;overflow:auto;max-height:100vh}
  header.top{display:flex;align-items:center;gap:16px;padding:16px 30px;border-bottom:1px solid var(--line);position:sticky;top:0;background:rgba(245,245,247,.8);backdrop-filter:saturate(180%) blur(20px);z-index:5}
  header.top h2{font-size:18px;font-weight:600;letter-spacing:-.02em}
  header.top .spacer{flex:1}
  .content{padding:26px 30px;max-width:1080px;width:100%}

  .card{background:var(--panel);border:1px solid var(--line);border-radius:var(--r2);overflow:hidden;box-shadow:var(--shadow-sm)}
  .card .chead{display:flex;align-items:center;gap:11px;padding:16px 20px;border-bottom:1px solid var(--line)}
  .card .chead h3{font-size:14.5px;font-weight:600}
  .card .chead .spacer{flex:1}
  .card .cbody{padding:6px 0}

  table{width:100%;border-collapse:collapse}
  th{text-align:left;padding:11px 20px;color:var(--dim);font-size:11px;font-weight:600;text-transform:uppercase;letter-spacing:.04em;border-bottom:1px solid var(--line)}
  td{padding:13px 20px;border-bottom:1px solid var(--line);vertical-align:middle}
  tr:last-child td{border-bottom:0}
  tbody tr{transition:background .1s} tbody tr:hover{background:rgba(0,0,0,.025)}
  .mono{font-family:var(--mono);font-size:12.5px}
  .pill{display:inline-flex;align-items:center;gap:5px;padding:2px 9px;border-radius:20px;font-size:11.5px;font-weight:500;background:rgba(120,120,128,.14);color:var(--mut)}
  .pill.acc{color:var(--acc);background:var(--accbg)}
  .pill.ok{color:var(--ok);background:var(--okbg)}
  .pill.bad{color:var(--bad);background:var(--badbg)}
  .dim{color:var(--dim)}

  .stats{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:14px;margin-bottom:22px}
  .stat{background:var(--panel);border:1px solid var(--line);border-radius:var(--r2);padding:18px 20px;box-shadow:var(--shadow-sm)}
  .stat .ico{width:36px;height:36px;border-radius:9px;background:var(--accbg);color:var(--acc);display:flex;align-items:center;justify-content:center;margin-bottom:14px}
  .stat .num{font-size:28px;font-weight:600;letter-spacing:-.02em}
  .stat .lbl{color:var(--mut);font-size:13px}
  .codeblock{background:var(--bg);border:1px solid var(--line);border-radius:8px;padding:12px 14px;font-family:var(--mono);font-size:12.5px;color:var(--fg);overflow:auto;display:flex;align-items:center;gap:10px}
  .codeblock .t{flex:1;word-break:break-all;color:var(--fg)}
  .empty{text-align:center;color:var(--dim);padding:48px 20px}
  .empty svg{width:32px;height:32px;margin-bottom:10px;stroke:var(--dim)}

  /* modal */
  .scrim{position:fixed;inset:0;background:rgba(0,0,0,.28);backdrop-filter:blur(2px);display:flex;align-items:flex-start;justify-content:center;padding:54px 20px;z-index:50;overflow:auto}
  .modal{width:100%;max-width:560px;background:var(--panel);border-radius:var(--r2);box-shadow:var(--shadow);animation:pop .18s ease}
  @keyframes pop{from{opacity:0;transform:translateY(-8px) scale(.98)}to{opacity:1;transform:none}}
  .modal .mhead{display:flex;align-items:center;padding:18px 22px;border-bottom:1px solid var(--line)}
  .modal .mhead h3{font-size:15.5px;font-weight:600;flex:1}
  .modal .mhead .x{color:var(--mut);padding:5px;border-radius:6px}.modal .mhead .x:hover{background:var(--panel2);color:var(--fg)}
  .modal .mbody{padding:20px 22px;display:grid;grid-template-columns:1fr 1fr;gap:14px}
  .modal .mbody .full{grid-column:1/-1}
  .modal .mfoot{display:flex;justify-content:flex-end;gap:10px;padding:14px 22px;border-top:1px solid var(--line)}
  .fielddesc{font-size:11.5px;color:var(--dim);margin-top:5px}

  /* toast */
  #toasts{position:fixed;top:18px;right:18px;display:flex;flex-direction:column;gap:10px;z-index:100}
  .toast{display:flex;align-items:center;gap:10px;background:var(--panel);border:1px solid var(--line);border-left:3px solid var(--acc);border-radius:10px;padding:12px 16px;min-width:240px;max-width:380px;box-shadow:var(--shadow);animation:slidein .25s ease}
  .toast.ok{border-left-color:var(--ok)} .toast.ok svg{color:var(--ok)}
  .toast.err{border-left-color:var(--bad)} .toast.err svg{color:var(--bad)}
  @keyframes slidein{from{opacity:0;transform:translateX(20px)}to{opacity:1;transform:none}}
  @media(max-width:760px){#app{grid-template-columns:1fr}aside{display:none}.modal .mbody{grid-template-columns:1fr}}
</style>
</head>
<body>
<div id="root"></div>
<div id="modal-root"></div>
<div id="toasts"></div>

<script>
//// icons ////////////////////////////////////////////////////////////////
const I = {
  logo:'<svg viewBox="0 0 24 24"><path d="M4 12h16M4 12l4-4M4 12l4 4M20 12l-4-4M20 12l-4 4"/></svg>',
  dash:'<svg viewBox="0 0 24 24"><rect x="3" y="3" width="7" height="9"/><rect x="14" y="3" width="7" height="5"/><rect x="14" y="12" width="7" height="9"/><rect x="3" y="16" width="7" height="5"/></svg>',
  server:'<svg viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="7" rx="2"/><rect x="3" y="13" width="18" height="7" rx="2"/><path d="M7 7.5h.01M7 16.5h.01"/></svg>',
  key:'<svg viewBox="0 0 24 24"><circle cx="7.5" cy="15.5" r="4"/><path d="M10.5 12.5L20 3M16 7l3 3M14 9l2 2"/></svg>',
  logout:'<svg viewBox="0 0 24 24"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9"/></svg>',
  plus:'<svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg>',
  trash:'<svg viewBox="0 0 24 24"><path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/></svg>',
  copy:'<svg viewBox="0 0 24 24"><rect x="9" y="9" width="12" height="12" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/></svg>',
  x:'<svg viewBox="0 0 24 24"><path d="M18 6L6 18M6 6l12 12"/></svg>',
  check:'<svg viewBox="0 0 24 24"><path d="M20 6L9 17l-5-5"/></svg>',
  alert:'<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="M12 8v4M12 16h.01"/></svg>',
  inbox:'<svg viewBox="0 0 24 24"><path d="M3 12h5l2 3h4l2-3h5M5 5h14a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z"/></svg>',
  link:'<svg viewBox="0 0 24 24"><path d="M10 13a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1 1M14 11a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1-1"/></svg>',
};

//// i18n /////////////////////////////////////////////////////////////////
const T = {
  en:{
    subtitle:'SSH-over-MCP Console',
    login_title:'Sign in', login_desc:'Enter the admin password to manage this Conduit instance.',
    password:'Admin password', sign_in:'Sign in', login_err:'Wrong admin password',
    nav_dash:'Dashboard', nav_servers:'Servers', nav_tokens:'Tokens', logout:'Log out',
    dash_title:'Dashboard', servers_title:'Servers', tokens_title:'Tokens',
    stat_servers:'Servers', stat_tokens:'Active tokens',
    mcp_card:'MCP endpoint', mcp_desc:'Point any MCP client here, authenticating with a token you create on the Tokens page.',
    connect_hdr:'Connect header',
    add_server:'Add server', create_token:'Create token',
    th_alias:'Alias', th_host:'Host', th_user:'User', th_auth:'Auth', th_jump:'Jump', th_tags:'Tags', th_actions:'',
    th_id:'ID', th_label:'Label', th_created:'Created', th_lastused:'Last used', th_status:'Status',
    no_servers:'No servers yet. Add one to get started.', no_tokens:'No tokens yet. Create one for your MCP client.',
    status_active:'active', status_revoked:'revoked',
    delete:'Delete', revoke:'Revoke',
    confirm_del_server:'Delete server "{0}"?', confirm_revoke:'Revoke token #{0}?',
    m_add_server:'Add server', m_create_token:'Create token',
    f_alias:'Alias', f_host:'Host', f_port:'Port', f_user:'Username', f_auth:'Authentication',
    f_secret_pw:'Password', f_secret_key:'Private key (OpenSSH PEM)',
    f_pass:'Key passphrase', f_pass_d:'Only if the private key is encrypted.',
    f_cert:'Certificate', f_cert_d:'Paste the *-cert.pub contents (CA-signed OpenSSH certificate).',
    f_jump:'Jump host alias', f_jump_d:'Optional — alias of a bastion to hop through first.',
    f_desc:'Description', f_tags:'Tags', f_tags_d:'Comma-separated, e.g. prod,web.',
    f_label:'Label', f_label_d:'A name to recognize this token, e.g. claude-desktop.',
    auth_pw:'Password', auth_key:'Private key', auth_cert:'Certificate (CA-signed)',
    cancel:'Cancel', save:'Save', create:'Create',
    token_created:'Token created', token_once:'Copy it now — it is shown only once and stored hashed.',
    copy:'Copy', copied:'Copied', close:'Close',
    saved:'Saved', deleted:'Deleted', revoked:'Revoked',
  },
  zh:{
    subtitle:'SSH-over-MCP 控制台',
    login_title:'登录', login_desc:'输入管理密码以管理此 Conduit 实例。',
    password:'管理密码', sign_in:'登录', login_err:'管理密码错误',
    nav_dash:'概览', nav_servers:'服务器', nav_tokens:'令牌', logout:'退出登录',
    dash_title:'概览', servers_title:'服务器', tokens_title:'令牌',
    stat_servers:'服务器', stat_tokens:'有效令牌',
    mcp_card:'MCP 端点', mcp_desc:'把任意 MCP 客户端指向这里，并用「令牌」页面创建的令牌进行认证。',
    connect_hdr:'认证请求头',
    add_server:'添加服务器', create_token:'创建令牌',
    th_alias:'别名', th_host:'主机', th_user:'用户', th_auth:'认证', th_jump:'跳板', th_tags:'标签', th_actions:'',
    th_id:'ID', th_label:'名称', th_created:'创建时间', th_lastused:'最近使用', th_status:'状态',
    no_servers:'还没有服务器，添加一个开始吧。', no_tokens:'还没有令牌，为你的 MCP 客户端创建一个。',
    status_active:'有效', status_revoked:'已吊销',
    delete:'删除', revoke:'吊销',
    confirm_del_server:'确认删除服务器「{0}」？', confirm_revoke:'确认吊销令牌 #{0}？',
    m_add_server:'添加服务器', m_create_token:'创建令牌',
    f_alias:'别名', f_host:'主机', f_port:'端口', f_user:'用户名', f_auth:'认证方式',
    f_secret_pw:'密码', f_secret_key:'私钥（OpenSSH PEM）',
    f_pass:'私钥口令', f_pass_d:'仅当私钥被加密时填写。',
    f_cert:'证书', f_cert_d:'粘贴 *-cert.pub 内容（CA 签发的 OpenSSH 证书）。',
    f_jump:'跳板机别名', f_jump_d:'可选 —— 先经过的跳板机别名。',
    f_desc:'描述', f_tags:'标签', f_tags_d:'逗号分隔，如 prod,web。',
    f_label:'名称', f_label_d:'便于识别此令牌的名字，如 claude-desktop。',
    auth_pw:'密码', auth_key:'私钥', auth_cert:'证书（CA 签发）',
    cancel:'取消', save:'保存', create:'创建',
    token_created:'令牌已创建', token_once:'请立即复制 —— 仅显示这一次，库中只存哈希。',
    copy:'复制', copied:'已复制', close:'关闭',
    saved:'已保存', deleted:'已删除', revoked:'已吊销',
  }
};

//// state ////////////////////////////////////////////////////////////////
const S = {
  pw: localStorage.getItem('cdt_pw') || '',
  lang: localStorage.getItem('cdt_lang') || (navigator.language.startsWith('zh')?'zh':'en'),
  view: 'dash',
  authed: false,
  servers: [], tokens: [],
};
const t = (k,...a)=>{ let s=(T[S.lang]&&T[S.lang][k])||k; a.forEach((v,i)=>s=s.replace('{'+i+'}',v)); return s; };
const esc = s => (s==null?'':String(s)).replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
const $ = (s,r=document)=>r.querySelector(s);

//// api //////////////////////////////////////////////////////////////////
async function api(method, path, body){
  const r = await fetch(path,{method,headers:{'Authorization':'Bearer '+S.pw,'Content-Type':'application/json'},body:body?JSON.stringify(body):undefined});
  if(r.status===401){ throw {unauth:true}; }
  const d = await r.json().catch(()=>({}));
  if(!r.ok) throw new Error(typeof d==='object'&&d.error?d.error:('HTTP '+r.status));
  return d;
}

function toast(msg, kind='ok'){
  const el=document.createElement('div'); el.className='toast '+kind;
  el.innerHTML=(kind==='err'?I.alert:I.check)+'<span>'+esc(msg)+'</span>';
  $('#toasts').appendChild(el);
  setTimeout(()=>{el.style.opacity='0';el.style.transition='opacity .3s';setTimeout(()=>el.remove(),300);},3200);
}

//// login ////////////////////////////////////////////////////////////////
function renderLogin(err){
  $('#root').innerHTML = `
  <div id="login"><div class="login-card">
    <div class="logo"><div class="mark">${I.logo}</div><div>
      <h1>Conduit</h1></div></div>
    <div class="sub">${t('subtitle')}</div>
    <form id="lf">
      <div>
        <label>${t('password')}</label>
        <input type="password" id="lpw" autofocus autocomplete="current-password">
        ${err?`<div class="fielddesc" style="color:var(--bad);margin-top:8px">${t('login_err')}</div>`:''}
      </div>
      <button class="btn" type="submit">${I.logout}${t('sign_in')}</button>
    </form>
    <div style="margin-top:20px"><div class="langtoggle">
      <button class="${S.lang==='en'?'on':''}" data-lang="en">EN</button>
      <button class="${S.lang==='zh'?'on':''}" data-lang="zh">中文</button>
    </div></div>
  </div></div>`;
  $('#lf').onsubmit=async e=>{
    e.preventDefault(); S.pw=$('#lpw').value;
    try{ await api('GET','/api/me'); localStorage.setItem('cdt_pw',S.pw); S.authed=true; boot(); }
    catch(_){ S.pw=''; renderLogin(true); }
  };
  document.querySelectorAll('[data-lang]').forEach(b=>b.onclick=()=>setLang(b.dataset.lang,true));
}

//// shell ////////////////////////////////////////////////////////////////
const NAV=[['dash',()=>I.dash,'nav_dash'],['servers',()=>I.server,'nav_servers'],['tokens',()=>I.key,'nav_tokens']];
function renderShell(){
  $('#root').innerHTML=`
  <div id="app">
    <aside>
      <div class="brand"><div class="mark">${I.logo}</div><div>
        <div class="name">Conduit</div><div class="tag">${t('subtitle')}</div></div></div>
      <nav>${NAV.map(([k,ic,lbl])=>`<button class="navitem ${S.view===k?'active':''}" data-view="${k}">${ic()}<span>${t(lbl)}</span></button>`).join('')}</nav>
      <div class="foot">
        <div class="langtoggle">
          <button class="${S.lang==='en'?'on':''}" data-lang="en">EN</button>
          <button class="${S.lang==='zh'?'on':''}" data-lang="zh">中文</button>
        </div>
        <button class="navitem" id="logout">${I.logout}<span>${t('logout')}</span></button>
      </div>
    </aside>
    <main>
      <header class="top"><h2 id="vtitle"></h2><div class="spacer"></div><div id="vaction"></div></header>
      <div class="content" id="view"></div>
    </main>
  </div>`;
  document.querySelectorAll('[data-view]').forEach(b=>b.onclick=()=>{S.view=b.dataset.view;renderShell();renderView();});
  document.querySelectorAll('[data-lang]').forEach(b=>b.onclick=()=>setLang(b.dataset.lang,false));
  $('#logout').onclick=()=>{localStorage.removeItem('cdt_pw');S.pw='';S.authed=false;renderLogin();};
  renderView();
}

function mcpUrl(){ return location.origin+'/mcp'; }

async function renderView(){
  const titleMap={dash:'dash_title',servers:'servers_title',tokens:'tokens_title'};
  $('#vtitle').textContent=t(titleMap[S.view]);
  const va=$('#vaction'); va.innerHTML='';
  if(S.view==='servers'){const b=document.createElement('button');b.className='btn';b.innerHTML=I.plus+t('add_server');b.onclick=openServerModal;va.appendChild(b);}
  if(S.view==='tokens'){const b=document.createElement('button');b.className='btn';b.innerHTML=I.plus+t('create_token');b.onclick=openTokenModal;va.appendChild(b);}
  try{
    if(S.view==='dash') await viewDash();
    if(S.view==='servers') await viewServers();
    if(S.view==='tokens') await viewTokens();
  }catch(e){ if(e&&e.unauth){bounce();return;} toast(e.message||'error','err'); }
}

async function viewDash(){
  const [sv,tk]=await Promise.all([api('GET','/api/servers'),api('GET','/api/tokens')]);
  S.servers=sv.servers; S.tokens=tk.tokens;
  const active=S.tokens.filter(x=>!x.revoked_at).length;
  $('#view').innerHTML=`
    <div class="stats">
      <div class="stat"><div class="ico">${I.server}</div><div class="num">${S.servers.length}</div><div class="lbl">${t('stat_servers')}</div></div>
      <div class="stat"><div class="ico">${I.key}</div><div class="num">${active}</div><div class="lbl">${t('stat_tokens')}</div></div>
    </div>
    <div class="card"><div class="chead"><div class="ico" style="color:var(--acc2)">${I.link}</div><h3>${t('mcp_card')}</h3></div>
      <div style="padding:20px 22px">
        <div class="codeblock" style="margin-bottom:14px"><span class="t">${esc(mcpUrl())}</span><button class="btn ghost sm" data-copy="${esc(mcpUrl())}">${I.copy}${t('copy')}</button></div>
        <p style="color:var(--mut);margin-bottom:14px">${t('mcp_desc')}</p>
        <label>${t('connect_hdr')}</label>
        <div class="codeblock"><span class="t">Authorization: Bearer &lt;token&gt;</span></div>
      </div></div>`;
  bindCopy();
}

async function viewServers(){
  const {servers}=await api('GET','/api/servers'); S.servers=servers;
  const rows = servers.length ? servers.map(s=>`<tr>
      <td><b>${esc(s.alias)}</b>${s.description?`<div class="dim" style="font-size:12px">${esc(s.description)}</div>`:''}</td>
      <td class="mono">${esc(s.host)}:${s.port}</td>
      <td class="mono">${esc(s.username)}</td>
      <td><span class="pill acc">${esc(s.auth_kind)}</span>${s.has_certificate?` <span class="pill">cert</span>`:''}${s.has_passphrase?` <span class="pill">passphrase</span>`:''}</td>
      <td>${s.jump_host_alias?`<span class="mono">${esc(s.jump_host_alias)}</span>`:'<span class="dim">—</span>'}</td>
      <td>${s.tags?esc(s.tags):'<span class="dim">—</span>'}</td>
      <td style="text-align:right"><button class="btn danger" data-del="${esc(s.alias)}">${I.trash}${t('delete')}</button></td></tr>`).join('')
    : `<tr><td colspan="7"><div class="empty">${I.inbox}<div>${t('no_servers')}</div></div></td></tr>`;
  $('#view').innerHTML=`<div class="card"><table><thead><tr>
    <th>${t('th_alias')}</th><th>${t('th_host')}</th><th>${t('th_user')}</th><th>${t('th_auth')}</th><th>${t('th_jump')}</th><th>${t('th_tags')}</th><th></th>
    </tr></thead><tbody>${rows}</tbody></table></div>`;
  document.querySelectorAll('[data-del]').forEach(b=>b.onclick=()=>delServer(b.dataset.del));
}

async function viewTokens(){
  const {tokens}=await api('GET','/api/tokens'); S.tokens=tokens;
  const rows = tokens.length ? tokens.map(x=>`<tr>
      <td class="mono dim">#${x.id}</td>
      <td><b>${esc(x.label)}</b></td>
      <td class="dim mono" style="font-size:12px">${esc((x.created_at||'').slice(0,19).replace('T',' '))}</td>
      <td class="dim mono" style="font-size:12px">${x.last_used_at?esc(x.last_used_at.slice(0,19).replace('T',' ')):'—'}</td>
      <td>${x.revoked_at?`<span class="pill bad">${t('status_revoked')}</span>`:`<span class="pill ok">${t('status_active')}</span>`}</td>
      <td style="text-align:right">${x.revoked_at?'':`<button class="btn danger" data-rev="${x.id}">${I.trash}${t('revoke')}</button>`}</td></tr>`).join('')
    : `<tr><td colspan="6"><div class="empty">${I.inbox}<div>${t('no_tokens')}</div></div></td></tr>`;
  $('#view').innerHTML=`<div class="card"><table><thead><tr>
    <th>${t('th_id')}</th><th>${t('th_label')}</th><th>${t('th_created')}</th><th>${t('th_lastused')}</th><th>${t('th_status')}</th><th></th>
    </tr></thead><tbody>${rows}</tbody></table></div>`;
  document.querySelectorAll('[data-rev]').forEach(b=>b.onclick=()=>revokeTok(b.dataset.rev));
}

//// modals ///////////////////////////////////////////////////////////////
function closeModal(){ $('#modal-root').innerHTML=''; }
function modal(title, bodyHTML, footHTML){
  $('#modal-root').innerHTML=`<div class="scrim"><div class="modal">
    <div class="mhead"><h3>${title}</h3><button class="x" id="mx">${I.x}</button></div>
    <div class="mbody">${bodyHTML}</div>
    <div class="mfoot">${footHTML}</div></div></div>`;
  $('#mx').onclick=closeModal;
  $('.scrim').onclick=e=>{ if(e.target.classList.contains('scrim')) closeModal(); };
}

function openServerModal(){
  modal(t('m_add_server'), `
    <div><label>${t('f_alias')} *</label><input id="f_alias"></div>
    <div><label>${t('f_host')} *</label><input id="f_host"></div>
    <div><label>${t('f_port')}</label><input id="f_port" type="number" value="22"></div>
    <div><label>${t('f_user')} *</label><input id="f_user"></div>
    <div class="full"><label>${t('f_auth')}</label>
      <select id="f_auth"><option value="password">${t('auth_pw')}</option><option value="key">${t('auth_key')}</option><option value="cert">${t('auth_cert')}</option></select></div>
    <div class="full"><label id="f_secret_l">${t('f_secret_pw')} *</label><textarea id="f_secret"></textarea></div>
    <div class="full hidden" id="row_pass"><label>${t('f_pass')}</label><input id="f_pass" type="password"><div class="fielddesc">${t('f_pass_d')}</div></div>
    <div class="full hidden" id="row_cert"><label>${t('f_cert')}</label><textarea id="f_cert" placeholder="ssh-ed25519-cert-v01@openssh.com ..."></textarea><div class="fielddesc">${t('f_cert_d')}</div></div>
    <div class="full"><label>${t('f_jump')}</label><input id="f_jump"><div class="fielddesc">${t('f_jump_d')}</div></div>
    <div><label>${t('f_desc')}</label><input id="f_desc"></div>
    <div><label>${t('f_tags')}</label><input id="f_tags"><div class="fielddesc">${t('f_tags_d')}</div></div>
  `, `<button class="btn ghost" id="mcancel">${t('cancel')}</button><button class="btn" id="msave">${t('save')}</button>`);
  const sync=()=>{const k=$('#f_auth').value;
    $('#f_secret_l').textContent=(k==='password'?t('f_secret_pw'):t('f_secret_key'))+' *';
    $('#row_pass').classList.toggle('hidden',k==='password');
    $('#row_cert').classList.toggle('hidden',k!=='cert');};
  $('#f_auth').onchange=sync; sync();
  $('#mcancel').onclick=closeModal;
  $('#msave').onclick=async()=>{
    const o={alias:$('#f_alias').value,host:$('#f_host').value,port:parseInt($('#f_port').value||'22',10),
      username:$('#f_user').value,auth_kind:$('#f_auth').value,secret:$('#f_secret').value,
      key_passphrase:$('#f_pass').value,certificate:$('#f_cert').value,
      jump_host_alias:$('#f_jump').value,description:$('#f_desc').value,tags:$('#f_tags').value};
    try{ await api('POST','/api/servers',o); closeModal(); toast(t('saved')); viewServers(); }
    catch(e){ if(e&&e.unauth)return bounce(); toast(e.message,'err'); }
  };
}

function openTokenModal(){
  modal(t('m_create_token'),
    `<div class="full"><label>${t('f_label')} *</label><input id="f_label" autofocus><div class="fielddesc">${t('f_label_d')}</div></div>`,
    `<button class="btn ghost" id="mcancel">${t('cancel')}</button><button class="btn" id="mcreate">${t('create')}</button>`);
  $('#mcancel').onclick=closeModal;
  $('#mcreate').onclick=async()=>{
    try{ const {token}=await api('POST','/api/tokens',{label:$('#f_label').value}); showToken(token); viewTokens(); }
    catch(e){ if(e&&e.unauth)return bounce(); toast(e.message,'err'); }
  };
}

function showToken(token){
  modal(t('token_created'),
    `<div class="full"><div style="display:flex;gap:8px;align-items:center;color:var(--warn);font-size:13px;margin-bottom:12px">${I.alert}<span>${t('token_once')}</span></div>
      <div class="codeblock"><span class="t" id="tokval">${esc(token)}</span><button class="btn ghost sm" data-copy="${esc(token)}">${I.copy}${t('copy')}</button></div></div>`,
    `<button class="btn" id="mclose">${t('close')}</button>`);
  $('#mclose').onclick=closeModal; bindCopy();
}

//// actions //////////////////////////////////////////////////////////////
async function delServer(alias){ if(!confirm(t('confirm_del_server',alias)))return;
  try{ await api('DELETE','/api/servers/'+encodeURIComponent(alias)); toast(t('deleted')); viewServers(); }
  catch(e){ if(e&&e.unauth)return bounce(); toast(e.message,'err'); } }
async function revokeTok(id){ if(!confirm(t('confirm_revoke',id)))return;
  try{ await api('DELETE','/api/tokens/'+id); toast(t('revoked')); viewTokens(); }
  catch(e){ if(e&&e.unauth)return bounce(); toast(e.message,'err'); } }

function bindCopy(){ document.querySelectorAll('[data-copy]').forEach(b=>b.onclick=async()=>{
  try{ await navigator.clipboard.writeText(b.dataset.copy); const o=b.innerHTML; b.innerHTML=I.check+t('copied'); setTimeout(()=>b.innerHTML=o,1500);}catch(_){}});}

//// glue /////////////////////////////////////////////////////////////////
function setLang(l, atLogin){ S.lang=l; localStorage.setItem('cdt_lang',l); document.documentElement.lang=l; if(atLogin) renderLogin(); else renderShell(); }
function bounce(){ localStorage.removeItem('cdt_pw'); S.pw=''; S.authed=false; renderLogin(true); }
function boot(){ document.documentElement.lang=S.lang; renderShell(); }

(async function init(){
  document.documentElement.lang=S.lang;
  if(S.pw){ try{ await api('GET','/api/me'); S.authed=true; return boot(); }catch(_){ S.pw=''; localStorage.removeItem('cdt_pw'); } }
  renderLogin(false);
})();
</script>
</body>
</html>
"##;
