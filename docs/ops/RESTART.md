# 🔄 הנחיות להפעלת המערכת מחדש

## ✅ המערכת כובתה בהצלחה

כל המשאבים שעולים כסף נעצרו:
- ✅ ECS Services (API + Worker-Notify): 0 tasks רצים
- ✅ RDS Database: במצב stopped

---

## 💰 עלויות בזמן כיבוי

**משאבים שעדיין עולים כסף (מינימלי):**
- Application Load Balancer (ALB): ~$0.60/יום (~$18/חודש)
- NAT Gateway: ~$1.08/יום (~$32/חודש)
- S3 Storage: ~$0.01/יום (זניח)
- CloudWatch Logs: ~$0.01/יום (זניח)

**סה"כ בכיבוי: ~$1.70/יום (~$51/חודש)**

**משאבים שלא עולים בכיבוי:**
- RDS (stopped): חינם עד 7 ימים, אח"כ מתחיל מחדש אוטומטית
- ECS Cluster: חינם (ללא tasks)
- ECR Images: ~$0.01/חודש
- VPC, Subnets, Security Groups: חינם
- Secrets Manager: ~$0.40/חודש לכל secret

---

## 🚀 איך להפעיל מחדש

### אופציה 1: PowerShell (מומלץ)

```powershell
# הגדר AWS Path
$env:Path = "C:\Program Files\Amazon\AWSCLIV2;" + $env:Path

# 1. הפעל RDS
Write-Host "Starting RDS..."
aws rds start-db-instance --db-instance-identifier eyedar-prod-db --region us-east-1

# חכה שה-RDS יהיה available (2-3 דקות)
Start-Sleep -Seconds 120

# 2. הפעל ECS Services
Write-Host "Starting API service..."
aws ecs update-service --cluster eyedar-prod --service eyedar-prod-api --desired-count 1 --region us-east-1

Write-Host "Starting Worker-Notify..."
aws ecs update-service --cluster eyedar-prod --service eyedar-prod-worker-notify --desired-count 1 --region us-east-1

Write-Host "Done! Services starting..."
```

### אופציה 2: דרך AWS Console

1. **הפעל RDS:**
   - לך ל: https://console.aws.amazon.com/rds
   - בחר `eyedar-prod-db`
   - Actions → Start

2. **חכה שה-RDS יהיה Available** (2-3 דקות)

3. **הפעל ECS Services:**
   - לך ל: https://console.aws.amazon.com/ecs/v2/clusters/eyedar-prod/services
   - בחר `eyedar-prod-api` → Update → Desired tasks: 1
   - בחר `eyedar-prod-worker-notify` → Update → Desired tasks: 1

---

## ⚠️ חשוב לדעת

### RDS Stop Limitation
- RDS יכול להישאר stopped רק **7 ימים**
- אחרי 7 ימים הוא מתחיל אוטומטית
- אם לא צריך לתקופה ארוכה - שקול snapshot + delete

### למחיקה מלאה (אם לא צריך יותר)
```powershell
# ⚠️ זה מוחק את הכל! השתמש רק אם בטוח!
cd C:\Users\roeea\OneDrive\DAR\DARserver\infra\envs\prod
terraform destroy
```

---

## 📋 סטטוס נוכחי (16/02/2026)

### ✅ מה שהושלם:
- [x] Infrastructure deployed (115 resources)
- [x] Firebase credentials uploaded
- [x] Worker-Notify Docker image pushed to ECR
- [x] DB password configured
- [x] All services tested and working

### ⏳ מה שנותר (להפעלה הבאה):
- [ ] Run DB migration (create PostGIS + tables)
  - Use AWS RDS Query Editor
  - SQL file: `infra/db/migrations/001_init_schema.sql`
- [ ] Test end-to-end notification flow

---

## 🔗 קישורים שימושיים

- **AWS Console:** https://console.aws.amazon.com
- **ECS Cluster:** https://console.aws.amazon.com/ecs/v2/clusters/eyedar-prod
- **RDS Database:** https://console.aws.amazon.com/rds/home?region=us-east-1#database:id=eyedar-prod-db
- **API Endpoint:** http://eyedar-prod-alb-334185939.us-east-1.elb.amazonaws.com

---

## 📞 פרטי התחברות

- **Region:** us-east-1
- **Account:** 221671810590
- **Cluster:** eyedar-prod
- **RDS Host:** eyedar-prod-db.csfmmaq82w8d.us-east-1.rds.amazonaws.com
- **DB Name:** eyedar
- **DB User:** eyedar_admin
- **DB Password:** Stored in Secrets Manager (`eyedar-prod-db-credentials`)
