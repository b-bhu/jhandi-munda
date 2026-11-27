import re, random, math
SYM = {
 'heart':('Heart','<svg viewBox="0 0 24 24"><path fill="#b3382c" d="M12 21s-8.5-5.4-8.5-11.4A4.6 4.6 0 0 1 12 7.2a4.6 4.6 0 0 1 8.5 2.4C20.5 15.600 12 21 12 21z"/></svg>'),
 'spade':('Spade','<svg viewBox="0 0 24 24"><path fill="#141414" d="M12 2C8 8 3.500 10 3.500 14.200a4 4 0 0 0 7 2.600c0 2-.6 3.400-2 4.700h7c-1.400-1.300-2-2.700-2-4.700a4 4 0 0 0 7-2.600C20.500 10 16 8 12 2z"/></svg>'),
 'diamond':('Diamond','<svg viewBox="0 0 24 24"><path fill="#b3382c" d="M12 2l9 10-9 10-9-10z"/></svg>'),
 'club':('Club','<svg viewBox="0 0 24 24"><g fill="#141414"><circle cx="12" cy="7" r="4.200"/><circle cx="7" cy="14" r="4.200"/><circle cx="17" cy="14" r="4.200"/><path d="M12 12l2.500 9.500h-5z"/></g></svg>'),
 'crown':('Crown (munda)','<svg viewBox="0 0 24 24"><path fill="#c9971f" stroke="#7a5a10" stroke-width=".6" d="M2.500 18.500L4 7l5 5 3-7 3 7 5-5 1.500 11.500z"/><rect x="2.500" y="19" width="19" height="2.200" fill="#a67c15"/></svg>'),
 'flag':('Flag (jhandi)','<svg viewBox="0 0 24 24"><rect x="5" y="2" width="1.800" height="20" fill="#2a2a2a"/><path fill="#141414" d="M7 3.500c4-1.500 6 1.500 11 0l-2 4.500 2.500 4.500c-5 1.500-7-1.500-11.500 0z"/></svg>'),
}
ORDER=['heart','spade','diamond','club','crown','flag']  # face 1..6 (UI assumption: board order)
def svg(k): return SYM[k][1]
NAV=lambda a:'<header class="top"><span class="logo">jhandi munda<i>.</i></span><nav class="nav">'+''.join(f'<a class="{"on" if a==n else ""}">{n}</a>' for n in ['Play','Pool','Fairness','Replay','Docs','GitHub'])+'<span class="pill">5qPR…acm7</span></nav></header>'
def banner(route,txt): return f'<div class="shellbanner"><span class="tag">UI shell · mock data</span><span>{route}</span><span style="flex:1"></span><span>{txt}</span></div>'
FOOT='<footer class="foot"><span class="logo">jhandi munda</span><span class="fl"><a>Play</a><a>Pool</a><a>Fairness</a><a>Replay</a><a>Docs</a><a>GitHub</a></span><span>Devnet only · test MUSD · built for MagicBlock Solana Blitz v10</span></footer>'
def page(title,body,active,route,note):
    return f'''<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{title}</title>
<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Manrope:wght@400;500;600;700&family=Inter+Tight:wght@400;500;600&family=JetBrains+Mono:wght@400;500;600;700&display=swap" rel="stylesheet">
<style>@import url(base.css);@import url(app.css);</style></head><body>{NAV(active)}{banner(route,note)}<main class="page">{body}</main></body></html>'''
def inline_css(h):
    return h.replace('<style>@import url(base.css);@import url(app.css);</style>','<style>'+open('base.css').read()+open('app.css').read()+'</style>')
def tile(k,cls='',stk='',you='',cnt=''):
    return f'<div class="tile {cls}"><span class="nm">{SYM[k][0].split()[0]}</span>{svg(k)}'+(f'<span class="cnt">{cnt}</span>' if cnt else '')+(f'<span class="stk">{stk}</span>' if stk else '')+(f'<span class="you">{you}</span>' if you else '')+'</div>'
def board(tiles): return '<div class="cloth"><div class="bgrid">'+''.join(tiles)+'</div></div>'
def die(face,hit=False): return f'<div class="die {"hit" if hit else ""}">{svg(ORDER[face-1])}</div>'
def mini(k): return f'<span class="mini">{svg(k)}</span>'
MOCK='<span class="mock">Example</span>'

# ---------------- PLAY ----------------
def betpanel(state):
    on = state=='open'
    sy=''.join(f'<span class="sb {"on" if k=="spade" and on else ""}">{svg(k)}</span>' for k in ORDER)
    p=f'''<div class="box panel"><div class="ptitle"><span>Place a bet</span>{MOCK}</div>
<div class="small" style="margin-bottom:10px">1 · Pick a symbol</div><div class="syms">{sy}</div>
<div class="small" style="margin:18px 0 0">2 · Amount (min $1)</div>
<div class="field"><span class="small">MUSD</span><b>{"5.00" if on else "—"}</b></div>
<div class="chips"><span>$1</span><span class="{"on" if on else ""}">$5</span><span>$10</span><span>$25</span><span>MAX</span></div>
<div class="kv" style="margin-top:14px"><span>If 2 dice match</span><b>+2x profit (3x back)</b></div>
<div class="kv"><span>If 6 dice match</span><b>+100x profit (101x back)</b></div>
<div class="kv"><span>Fee</span><b>2% of a winner’s profit</b></div>
<span class="go {"" if on else "off"}">{"Place bet · 5 MUSD" if on else "Betting closed"}</span>
<p class="note"><span class="pill2 g">Session on</span> &nbsp;58 min left · bets need no wallet popup (cap 50 MUSD)</p></div>'''
    return p
def mybets(state):
    if state=='open':
        rows=[('spade','Spade','5.00 MUSD','Placed · no popup'),('heart','Heart','2.00 MUSD','Placed · no popup')]
        res=['Open','Open']
    elif state=='roll':
        rows=[('spade','Spade','5.00 MUSD','Locked'),('heart','Heart','2.00 MUSD','Locked')]; res=['Waiting','Waiting']
    else:
        rows=[('spade','Spade','5.00 MUSD','2 dice matched'),('heart','Heart','2.00 MUSD','0 dice matched')]; res=['Won +10.00','Lost −2.00']
    out='<div class="box panel"><div class="ptitle"><span>My bets this round</span>'+MOCK+'</div>'
    for (k,n,a,s),r in zip(rows,res):
        col='g' if r.startswith('Won') else ('r' if r.startswith('Lost') else '')
        out+=f'<div class="row">{mini(k)}<div><div style="font-weight:600">{n} · {a}</div><div class="s">{s}</div></div><span class="a kv"><b class="{col}" style="border:0">{r}</b></span></div>'
    return out+'</div>'
def play():
    # A open
    ta=[tile('heart','',stk='$14',you='You $2'),tile('spade','sel',stk='$31',you='You $5'),tile('diamond','',stk='$6'),tile('club','',stk='$9'),tile('crown','',stk='$22'),tile('flag','',stk='$3')]
    A=f'''<div class="state"><div class="stlabel"><b>State A</b>Betting open<span class="x"></span></div><div class="grid2"><div>
<div class="timer"><span class="big">01:12</span><div style="flex:1"><div class="bar"><i style="width:20%"></i></div><div class="small" style="margin-top:8px">Window closes in 72 s of 90 s · 4 players at the table {MOCK}</div></div></div>{board(ta)}
<p class="note">Cloth totals show all players’ bets on each symbol (example). The gold tile is the symbol you picked.</p></div>
<div style="display:grid;gap:24px">{betpanel('open')}{mybets('open')}</div></div></div>'''
    tb=[tile('heart','dim',stk='$14',you='You $2'),tile('spade','dim',stk='$31',you='You $5'),tile('diamond','dim',stk='$6'),tile('club','dim',stk='$9'),tile('crown','dim',stk='$22'),tile('flag','dim',stk='$3')]
    B=f'''<div class="state"><div class="stlabel"><b>State B</b>Bowl covered · rolling<span class="x"></span></div><div class="grid2"><div>
<div class="timer"><span class="big">00:00</span><div style="flex:1"><div class="bar"><i style="width:100%"></i></div><div class="small" style="margin-top:8px">Window closed · bets are locked {MOCK}</div></div></div>
<div class="bowl"><div class="lab"><span class="spin"></span>The dealer has covered the bowl. Waiting for the random number (VRF)…</div></div>
<div class="dice" style="margin-top:18px">{''.join('<div class="die q">?</div>' for _ in range(6))}</div>
<p class="note">If you open the page late, you still see the result, never a blank board (#58).</p></div>
<div style="display:grid;gap:24px">{betpanel('roll')}{mybets('roll')}</div></div></div>'''
    faces=[2,2,4,6,4,5]
    cnt={i:faces.count(i) for i in range(1,7)}
    tc=[]
    for i,k in enumerate(ORDER,1):
        c=cnt[i]; tc.append(tile(k,'hit' if c>=2 else 'dim',cnt=f'×{c}',you={1:'You $2',2:'You $5'}.get(i,'')))
    C=f'''<div class="state"><div class="stlabel"><b>State C</b>Result and claims<span class="x"></span></div><div class="grid2"><div>
<div class="dice" style="margin-bottom:18px">{''.join(die(f,cnt[f]>=2) for f in faces)}</div>
{board(tc)}<p class="note">Faces {MOCK}: spade ×2, club ×2, others fewer. Two or more of a symbol wins for that symbol. Faces link to the fairness page.</p></div>
<div style="display:grid;gap:24px">{mybets('result')}
<div class="box panel"><div class="ptitle"><span>Math for this round</span>{MOCK}</div>
<div class="kv"><span>Spade $5 · 2 matched · 2x profit</span><b>+10.00</b></div><div class="kv"><span>2% fee on profit</span><b>−0.20</b></div><div class="kv"><span>You get back (stake included)</span><b class="g">14.80</b></div>
<div class="kv"><span>Heart $2 · 0 matched</span><b class="r">−2.00</b></div></div></div></div>
<div class="claims">
<div class="box claim"><span class="tag" style="color:var(--gold)">You won</span><h3>14.80 MUSD ready</h3><p>Wait 90 seconds for the full payout, or take 95% now. If the pool is short, your win joins the first-in, first-out queue.</p>
<div class="acts"><span class="b1">Wait · 14.80 in 90 s</span><span class="b2">Take now · 14.06 (95%)</span></div><p class="note">Example numbers.</p></div>
<div class="box claim"><span class="tag" style="color:var(--tilak)">You lost</span><h3>You earned 10 MUNDA</h3><p>MUNDA is a share of house fees. This is not a refund. It goes straight into your stake after 24 hours. Your first $1,000 of losses earn 5x MUNDA (2 × 5 = 10). Chip bets earn it too.</p>
<div class="acts"><span class="b1">Wait · 24 h → stake</span><span class="b2 off">Take now · not for losses</span></div><p class="note">Example numbers. D50, D51.</p></div></div></div>'''
    body=f'<div class="ph"><div class="rule"><span class="l"><b>TABLE 1</b> Round 13 {MOCK}</span><span class="x"></span></div><h1>Pick a symbol. <em>Watch the bowl.</em></h1><p class="lead">Public 90-second table on a MagicBlock Ephemeral Rollup. Three states shown below: betting open, bowl covered, and the result with claims.</p></div>{A}{B}{C}{FOOT}'
    return page('Play — mock',body,'Play','/play','Round, bets and totals are example data')
# ---------------- POOL ----------------
def spark(vals,w=520,h=140,col='#f2b84b'):
    mn,mx=min(vals),max(vals); pts=[(i*w/(len(vals)-1),h-8-(v-mn)/(mx-mn)*(h-24)) for i,v in enumerate(vals)]
    return f'<svg viewBox="0 0 {w} {h}" width="100%"><polyline fill="none" stroke="{col}" stroke-width="2" points="{" ".join(f"{x:.1f},{y:.1f}" for x,y in pts)}"/>'+''.join(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="3" fill="{col}"/>' for x,y in pts)+'</svg>'
def pool():
    q=[(1,'Win · round 9','2.96','Next in line'),(2,'Win · round 10','6.40','Behind #1'),(3,'Win · round 11','1.10','Behind #2')]
    qrows=''.join(f'<tr><td class="m">#{a}</td><td>{b}</td><td class="m">{c} MUSD</td><td><span class="pill2 {"o" if a==1 else ""}">{d}</span></td></tr>' for a,b,c,d in q)
    lots=[('Lot 1','3.2 days ago','100 MUNDA','g'),('Lot 2','1.1 days ago','40 MUNDA','g'),('Lot 3','6 h ago','25 MUNDA','o')]
    lt=''.join(f'<div class="lot"><span class="t">{a}</span><span>{b} · {c}</span><span class="pill2 {"g" if k=="g" else "o"}">{"earning fees" if k=="g" else "18 h lock left"}</span></div>' for a,b,c,k in lots)
    body=f'''<div class="ph"><div class="rule"><span class="l"><b>POOL</b> The house {MOCK}</span><span class="x"></span></div><h1>Anyone can be <em>the house.</em></h1><p class="lead">Players bet against this shared pool. Deposit test MUSD, get shares, and watch what they are worth. Pool value can go down as well as up.</p></div>
<div class="state" style="margin-top:32px"><div class="grid2" style="grid-template-columns:1fr 1fr">
<div class="box panel"><div class="ptitle"><span>Pool NAV · per share</span>{MOCK}</div><div class="bigval">$1.25 <small>per share</small></div>
<div class="formula">NAV = (vault − owed to queue) ÷ shares<br>= (1,262.00 − 12.00 owed) ÷ 1,000<br>= 1.25</div>
<div class="kv" style="margin-top:16px"><span>Vault (MUSD)</span><b>1,262.00</b></div><div class="kv"><span>Owed to the payout queue</span><b>12.00</b></div><div class="kv"><span>Pool shares outstanding</span><b>1,000</b></div><div class="kv"><span>Pool assets (vault − owed)</span><b>1,250.00</b></div>
<p class="note">Queued wins are IOUs, not pool money (D44). All numbers here are examples; the real ones are read from the devnet pool (#10).</p></div>
<div class="box panel"><div class="tabs"><span class="on">Deposit</span><span>Withdraw</span></div><div class="ptitle"><span>Deposit MUSD, get shares</span>{MOCK}</div>
<div class="field"><span class="small">Amount</span><b>100.00</b></div><div class="chips"><span>10</span><span>50</span><span class="on">100</span><span>MAX</span></div>
<div class="kv" style="margin-top:14px"><span>Your wallet</span><b>340.00 MUSD</b></div><div class="kv"><span>You receive (at NAV 1.25)</span><b>≈ 80.00 shares</b></div><span class="go">Deposit 100 MUSD</span>
<p class="note">Withdraw burns shares at today’s NAV, rounded in the pool’s favour. Need test MUSD? Use the <u>faucet</u>.</p></div></div></div>
<div class="state" style="margin-top:24px"><div class="grid2" style="grid-template-columns:1fr 1fr">
<div class="box panel"><div class="ptitle"><span>My LP position</span>{MOCK}</div><div class="kv"><span>Shares</span><b>80.00</b></div><div class="kv"><span>You bought at</span><b>$1.00 / share</b></div><div class="kv"><span>Worth now</span><b>$1.25 / share → $100.00</b></div><div class="kv"><span>Your profit</span><b class="g">+25.0%</b></div></div>
<div class="box panel"><div class="ptitle"><span>Payout queue · first in, first out</span>{MOCK}</div><table class="tbl"><tr><th>#</th><th>What</th><th>Owed</th><th>Status</th></tr>{qrows}</table><p class="note">When the pool can’t pay a win, it waits here and is paid from future losses. No skipping (D30). The queue holds up to 100 wins.</p></div></div></div>
<div class="state" style="margin-top:24px"><div class="grid2" style="grid-template-columns:1fr 1fr">
<div class="box panel"><div class="ptitle"><span>My MUNDA stake</span>{MOCK}</div><div class="bigval" style="font-size:40px">165 <small>MUNDA staked</small></div>
<div style="margin-top:14px">{lt}</div>
<div class="kv" style="margin-top:6px"><span>Worth when you got it</span><b>$3.30</b></div><div class="kv"><span>Worth now</span><b class="g">$3.80</b></div>
<div class="lock"><b>Locked</b><span><b style="color:var(--t1)">Unstake is off.</b> The admin “transfers” switch is not on yet (D46, D47), so MUNDA stays in your stake. New MUNDA earns nothing for its first 24 hours (D29). This is not a refund of lost money.</span></div>
<span class="go off" style="margin-top:14px">Unstake · disabled until transfers are on</span></div>
<div class="box panel"><div class="ptitle"><span>Daily snapshots · NAV per share</span>{MOCK}</div>{spark([1.00,1.02,1.01,1.07,1.12,1.18,1.25])}
<div class="legend"><span>7 daily snapshots · example</span></div>
<div class="kv" style="margin-top:14px"><span>Fee split</span><b>70% stakers · 20% insurance · 10% devs</b></div><div class="kv"><span>Above $10,000 in the pool</span><b>gains split 50/50 stakers and LPs</b></div><p class="note">Hackathon demo on devnet. Test money only. Nothing here is a promise of returns.</p></div></div></div>{FOOT}'''
    return page('Pool — mock',body,'Pool','/pool','Every number is example data')
# ---------------- FAIRNESS ----------------
def fairness():
    raw=open('/home/box/work/jhandi-munda-casino/docs/deploys.md').read()
    names=['place_bet_chips','request_roll','VRF callback','settle_chip_bets','deposit','pay_queue','finish_round']
    sig={}
    for n in names:
        m=re.search(re.escape(n)+r'[^\[]*: \[(\w{8})…\]\(https://explorer.solana.com/tx/(\w+)\?cluster=devnet\)',raw)
        sig[n]=m.group(2)
    desc=[('place_bet_chips','Bet placed','A chip bet is placed on the Ephemeral Rollup.'),('request_roll','Roll requested','The program asks MagicBlock’s random number service (VRF) for six dice.'),('VRF callback','Dice rolled · VRF callback','The service answers. The six faces are written onchain.'),('settle_chip_bets','Bets settled','Each bet is checked against the faces.'),('deposit','Pool top-up','A deposit gave the pool cash to pay the queue.'),('pay_queue','Queued win paid','The 2.96 MUSD win was paid, first in first out.'),('finish_round','Round finished','The round closes. A new one can start.')]
    lis=''.join(f'<li><span class="n">{i+1:02}</span><span class="w">{t}<small>{d}</small></span><span class="c">Solana devnet</span><a>{sig[n][:8]}…{sig[n][-4:]} ↗</a></li>' for i,(n,t,d) in enumerate(desc))
    faces=[2,2,4,6,4,5]; cnt={i:faces.count(i) for i in range(1,7)}
    rows=''.join(f'<tr><td>{mini(ORDER[i-1])}</td><td>{SYM[ORDER[i-1]][0]}</td><td class="m">face {i}</td><td class="m">{cnt[i]}</td><td class="m">{["lose","lose","2x profit (3x back)","4x (5x)","8x (9x)","25x (26x)","100x (101x)"][cnt[i]] if cnt[i]>=2 else "lose"}</td></tr>' for i in range(1,7))
    body=f'''<div class="ph"><div class="rule"><span class="l"><b>FAIRNESS</b> /fairness/12</span><span class="x"></span></div><h1>Round 12. <em>Check it yourself.</em></h1><p class="lead">Every settled round has a public page: the six faces, the bets, and the transactions. The dice come from MagicBlock’s random number service, not from us. Round and signatures below are real devnet data.</p></div>
<div class="state" style="margin-top:28px"><div class="grid2" style="grid-template-columns:1fr 1fr"><div class="box panel"><div class="ptitle"><span>The six faces</span><span class="mock" style="border-style:solid">Real · devnet</span></div>
<div class="dice">{''.join(die(f,cnt[f]>=2) for f in faces)}</div><p class="note" style="margin-top:16px">Faces [2, 2, 4, 6, 4, 5]. Faces are numbered 1 to 6; the picture mapping (board order) is a UI choice to confirm.</p></div>
<div class="box panel"><div class="ptitle"><span>Round summary</span><span class="mock" style="border-style:solid">Real · devnet</span></div><div class="kv"><span>Round</span><b>12</b></div><div class="kv"><span>Network</span><b>Solana devnet</b></div><div class="kv"><span>Bet</span><b>one chip bet · 2 dice matched</b></div><div class="kv"><span>Result</span><b class="g">won 2.96 MUSD</b></div><div class="kv"><span>Payout</span><b>queued, then paid (pay_queue)</b></div></div></div></div>
<div class="state" style="margin-top:24px"><div class="stlabel"><b>Proof</b>Transactions for round 12<span class="x"></span></div><ul class="proof">{lis}</ul><p class="note">Every link opens the Solana Explorer on devnet. A proof blob from the VRF, if available, will show here (#54).</p></div>
<div class="state" style="margin-top:24px"><div class="box panel"><div class="ptitle"><span>Matches per symbol</span><span class="mock" style="border-style:solid">Real · from faces</span></div><table class="tbl"><tr><th></th><th>Symbol</th><th>Face</th><th>Dice</th><th>Pays</th></tr>{rows}</table></div></div>{FOOT}'''
    return page('Fairness — mock',body,'Fairness','/fairness/12','Round 12 signatures are real (docs/deploys.md)')
# ---------------- REPLAY ----------------
def replay():
    random.seed(7); jack=31; BET=40; owed_total=BET*101
    cash=2000.0; P=[];Q=[];inc=[]
    owed=0.0; paid_now=0; queued0=0
    for r in range(1,51):
        if r<jack: cash+=random.uniform(-60,150); owed=0
        elif r==jack:
            paid_now=max(cash-60,0); queued0=owed_total-paid_now; cash=cash-paid_now; owed=queued0
        else:
            income=random.uniform(150,420)
            pay=min(owed,income); owed-=pay; cash+=income-pay
        P.append(cash); Q.append(owed)
    empty=next(i+1 for i in range(jack,50) if Q[i]<=0.5)
    W,H=760,260; mx=max(max(P),queued0)*1.08
    px=lambda i:40+i*(W-60)/49; py=lambda v:H-24-v/mx*(H-48)
    pts=' '.join(f'{px(i):.1f},{py(v):.1f}' for i,v in enumerate(P)); qp=' '.join(f'{px(i):.1f},{py(v):.1f}' for i,v in enumerate(Q))
    chart=f'<svg viewBox="0 0 {W} {H}" width="100%"><line x1="40" y1="{H-24}" x2="{W-20}" y2="{H-24}" stroke="#ffffff24"/>'+''.join(f'<text x="{px(i):.0f}" y="{H-6}" fill="#8a8a8a" font-size="10" font-family="JetBrains Mono" text-anchor="middle">{i+1}</text>' for i in (0,9,19,29,39,49))+f'<polyline fill="none" stroke="#f2b84b" stroke-width="2.2" points="{pts}"/><polyline fill="none" stroke="#e0483b" stroke-width="1.8" stroke-dasharray="4 3" points="{qp}"/><line x1="{px(jack-1):.1f}" y1="14" x2="{px(jack-1):.1f}" y2="{H-24}" stroke="#8fd3b6" stroke-dasharray="3 3"/><circle cx="{px(jack-1):.1f}" cy="{py(P[jack-1]):.1f}" r="6" fill="#8fd3b6"/><text x="{px(jack-1)-10:.1f}" y="26" fill="#8fd3b6" font-size="12" font-family="JetBrains Mono" font-weight="700" text-anchor="end">Round {jack}: 101x jackpot</text></svg>'
    f=lambda v:f"{v:,.0f}"
    body=f'''<div class="ph"><div class="rule"><span class="l"><b>REPLAY</b> 50-round simulation <span class="mock">Simulation · example data</span></span><span class="x"></span></div><h1>The pool takes a <em>101x hit.</em></h1><p class="lead">A replay of 50 simulated rounds. One player hits all six dice. The pool pays what it can, the rest waits in the queue, and the table keeps taking bets. This is a simulation, not live results.</p></div>
<div class="state" style="margin-top:28px"><div class="stats"><div><b>50</b><span>rounds replayed</span></div><div><b>1</b><span>101x jackpot (round {jack})</span></div><div><b>${f(min(P[jack-1:]))}</b><span>pool cash low point</span></div><div><b>{empty-jack}</b><span>rounds until the queue was empty</span></div></div></div>
<div class="state" style="margin-top:24px"><div class="grid2" style="grid-template-columns:1.6fr 1fr"><div class="box panel"><div class="ptitle"><span>Pool cash and queue by round</span><span class="mock">Simulation</span></div>{chart}
<div class="legend"><span><i style="background:#f2b84b"></i>Pool cash (MUSD)</span><span><i style="background:#e0483b"></i>Owed in queue (MUSD)</span><span><i style="background:#8fd3b6"></i>Jackpot</span></div>
<div class="scrub"><span class="play">▶</span><div class="bar"><i style="width:62%"></i></div><span class="small" style="font:600 13px var(--mono)">Round {jack} / 50</span></div></div>
<div style="display:grid;gap:24px"><div class="box panel"><div class="ptitle"><span>Round {jack}</span><span class="mock">Simulation</span></div><div class="dice">{''.join(die(5,True) for _ in range(6))}</div><div class="kv" style="margin-top:16px"><span>Bet</span><b>${BET} on Crown</b></div><div class="kv"><span>Dice matched</span><b>6 of 6</b></div><div class="kv"><span>Owed (101x back)</span><b class="g">${f(owed_total)}</b></div><div class="kv"><span>Paid now from the pool</span><b>${f(paid_now)}</b></div><div class="kv"><span>Waits in the queue</span><b>${f(queued0)}</b></div></div>
<div class="box panel"><div class="ptitle"><span>After the jackpot</span><span class="mock">Simulation</span></div><div class="kv"><span>Round {jack+1}</span><b>table open · queue owes ${f(Q[jack])}</b></div><div class="kv"><span>Round {jack+4}</span><b>queue owes ${f(Q[jack+3])}</b></div><div class="kv"><span>Round {empty}</span><b class="g">queue empty · bets kept coming</b></div></div></div></div></div>
<div class="state" style="margin-top:24px"><p class="note">Simulation data, not live results. The sim script (#63) will replace this fixture, and a jump control will let you pick any round.</p></div>{FOOT}'''
    return page('Replay — mock',body,'Replay','/replay','Simulation · example data')
for n,f in [('play',play),('pool',pool),('fairness',fairness),('replay',replay)]:
    open(f'{n}.html','w').write(f())
print('built')
