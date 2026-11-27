import subprocess,sys
from PIL import Image
def shot(name,out,w,h=9000):
    subprocess.run(['google-chrome','--headless=new','--no-sandbox','--disable-gpu','--hide-scrollbars',f'--window-size={w},{h}','--virtual-time-budget=8000',f'--screenshot=/tmp/{out}',f'file:///workspace/jhandi-site/app-mock/{name}.html'],capture_output=True)
    im=Image.open(f'/tmp/{out}').convert('RGB'); W,H=im.size; bg=(16,18,22)
    y=H-1
    while y>0 and all(im.getpixel((x,y))==bg for x in range(0,W,max(1,W//40))): y-=1
    im.crop((0,0,W,min(H,y+40))).save(f'/workspace/jhandi-site/{out}'); print(out,W,y+40)
shot('play','play.png',1440); shot('pool','pool.png',1440); shot('fairness','fairness.png',1440); shot('replay','replay.png',1440); shot('play','play-mobile.png',390,12000)
