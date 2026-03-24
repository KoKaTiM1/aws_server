# Push Worker-Notify to ECR - Manual Steps

התמונה נבנתה בהצלחה! עכשיו צריך לדחוף אותה ל-ECR.

## שלב 1: התחברות ל-ECR

פתח PowerShell או Command Prompt **חדש** (עם AWS CLI בנתיב) והרץ:

```powershell
# Get ECR login token
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin 221671810590.dkr.ecr.us-east-1.amazonaws.com
```

## שלב 2: תיוג התמונה

```powershell
docker tag eyedar-worker-notify:latest 221671810590.dkr.ecr.us-east-1.amazonaws.com/eyedar-prod-worker-notify:latest
```

## שלב 3: דחיפה ל-ECR

```powershell
docker push 221671810590.dkr.ecr.us-east-1.amazonaws.com/eyedar-prod-worker-notify:latest
```

## שלב 4: עדכון שירות ECS

```powershell
aws ecs update-service `
  --cluster eyedar-prod `
  --service eyedar-prod-worker-notify `
  --force-new-deployment `
  --region us-east-1
```

## שלב 5: בדיקת סטטוס

```powershell
# Check service status
aws ecs describe-services `
  --cluster eyedar-prod `
  --services eyedar-prod-worker-notify `
  --region us-east-1 `
  --query 'services[0].{Status:status,Running:runningCount,Desired:desiredCount}'

# Check logs
aws logs tail /ecs/eyedar-prod-worker-notify --follow --region us-east-1
```

---

## אם AWS CLI לא פועל

התקן AWS CLI מחדש:
- Windows: https://awscli.amazonaws.com/AWSCLIV2.msi
- אחרי התקנה, פתח terminal חדש והרץ: `aws configure`

---

## פרטים טכניים

- **Account ID**: 221671810590
- **Region**: us-east-1
- **ECR Repository**: eyedar-prod-worker-notify
- **ECS Cluster**: eyedar-prod
- **ECS Service**: eyedar-prod-worker-notify
- **Image**: 221671810590.dkr.ecr.us-east-1.amazonaws.com/eyedar-prod-worker-notify:latest
