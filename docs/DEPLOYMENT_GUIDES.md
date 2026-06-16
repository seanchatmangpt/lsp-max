# Deployment Guides for lsp-max

Complete step-by-step deployment instructions for lsp-max in various environments: Kubernetes, Docker Compose, AWS, GCP, GitHub Actions, and custom agent systems.

---

## 1. Kubernetes Deployment

### 1.1 Namespace Setup

Create a dedicated namespace and service account:

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: lsp-max
  labels:
    name: lsp-max

---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: lsp-max
  namespace: lsp-max

---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: lsp-max
rules:
  # Service discovery
  - apiGroups: [""]
    resources: ["services", "endpoints"]
    verbs: ["get", "list", "watch"]
  # ConfigMaps / Secrets
  - apiGroups: [""]
    resources: ["configmaps", "secrets"]
    verbs: ["get", "list", "watch"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: lsp-max
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: lsp-max
subjects:
  - kind: ServiceAccount
    name: lsp-max
    namespace: lsp-max
```

Apply:

```bash
kubectl apply -f namespace.yaml
```

### 1.2 ConfigMap & Secrets

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: lsp-max-config
  namespace: lsp-max
data:
  config.yaml: |
    server:
      bind: "0.0.0.0:8080"
      max_connections: 500
    logging:
      level: info
      format: json
    observability:
      otel_endpoint: "http://otel-collector.observability:4317"
      metrics:
        enabled: true
        prometheus_port: 9091

---
apiVersion: v1
kind: Secret
metadata:
  name: lsp-max-secrets
  namespace: lsp-max
type: Opaque
stringData:
  # Base64-encoded credentials for external systems
  git-token: "ghp_xxxxxxxxxxxxxxxxxxxx"
  otel-api-key: "your-api-key-here"
```

Apply:

```bash
kubectl apply -f configmap-secret.yaml
```

### 1.3 Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: lsp-max
  namespace: lsp-max
  labels:
    app: lsp-max
    version: v26.7
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: lsp-max
  template:
    metadata:
      labels:
        app: lsp-max
        version: v26.7
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9091"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: lsp-max
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      
      containers:
        - name: lsp-max
          image: gcr.io/my-project/lsp-max:26.7.1
          imagePullPolicy: IfNotPresent
          
          ports:
            - name: lsp
              containerPort: 8080
              protocol: TCP
            - name: metrics
              containerPort: 9091
              protocol: TCP
          
          env:
            - name: LSP_MAX_BIND_ADDRESS
              value: "0.0.0.0:8080"
            - name: LSP_MAX_LOG_LEVEL
              value: "info"
            - name: OTEL_EXPORTER_OTLP_ENDPOINT
              value: "http://otel-collector.observability:4317"
            - name: GIT_TOKEN
              valueFrom:
                secretKeyRef:
                  name: lsp-max-secrets
                  key: git-token
            - name: OTEL_API_KEY
              valueFrom:
                secretKeyRef:
                  name: lsp-max-secrets
                  key: otel-api-key
          
          volumeMounts:
            - name: config
              mountPath: /etc/lsp-max
              readOnly: true
            - name: session-tmp
              mountPath: /tmp/lsp-max-sessions
            - name: logs
              mountPath: /var/log/lsp-max
          
          livenessProbe:
            httpGet:
              path: /healthz
              port: 8080
            initialDelaySeconds: 30
            periodSeconds: 10
            timeoutSeconds: 5
            failureThreshold: 3
          
          readinessProbe:
            httpGet:
              path: /ready
              port: 8080
            initialDelaySeconds: 10
            periodSeconds: 5
            timeoutSeconds: 3
            failureThreshold: 2
          
          resources:
            requests:
              cpu: "500m"
              memory: "512Mi"
            limits:
              cpu: "2000m"
              memory: "2Gi"
          
          securityContext:
            allowPrivilegeEscalation: false
            capabilities:
              drop:
                - ALL
            readOnlyRootFilesystem: true
      
      volumes:
        - name: config
          configMap:
            name: lsp-max-config
        - name: session-tmp
          emptyDir:
            medium: Memory
            sizeLimit: 512Mi
        - name: logs
          emptyDir:
            sizeLimit: 1Gi
      
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchExpressions:
                    - key: app
                      operator: In
                      values:
                        - lsp-max
                topologyKey: kubernetes.io/hostname
```

Apply:

```bash
kubectl apply -f deployment.yaml
kubectl rollout status deployment/lsp-max -n lsp-max
```

### 1.4 Service & Ingress

```yaml
apiVersion: v1
kind: Service
metadata:
  name: lsp-max
  namespace: lsp-max
  labels:
    app: lsp-max
spec:
  type: ClusterIP
  ports:
    - name: lsp
      port: 8080
      targetPort: 8080
      protocol: TCP
    - name: metrics
      port: 9091
      targetPort: 9091
      protocol: TCP
  selector:
    app: lsp-max

---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: lsp-max
  namespace: lsp-max
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/websocket-services: lsp-max
    nginx.ingress.kubernetes.io/proxy-read-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "3600"
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - lsp-max.example.com
      secretName: lsp-max-tls
  rules:
    - host: lsp-max.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: lsp-max
                port:
                  number: 8080
```

Apply:

```bash
kubectl apply -f service-ingress.yaml
kubectl get ingress -n lsp-max
```

### 1.5 Network Policy

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: lsp-max
  namespace: lsp-max
spec:
  podSelector:
    matchLabels:
      app: lsp-max
  policyTypes:
    - Ingress
    - Egress
  ingress:
    # From agents/clients
    - from:
        - namespaceSelector:
            matchLabels:
              name: agents
      ports:
        - protocol: TCP
          port: 8080
    # From monitoring/scraping
    - from:
        - namespaceSelector:
            matchLabels:
              name: monitoring
      ports:
        - protocol: TCP
          port: 9091
    # From ingress controller
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress-nginx
      ports:
        - protocol: TCP
          port: 8080
  egress:
    # DNS
    - to:
        - namespaceSelector: {}
      ports:
        - protocol: UDP
          port: 53
    # OTel collector
    - to:
        - namespaceSelector:
            matchLabels:
              name: observability
      ports:
        - protocol: TCP
          port: 4317
    # Kubernetes API
    - to:
        - namespaceSelector:
            matchLabels:
              name: kube-system
      ports:
        - protocol: TCP
          port: 443
```

Apply:

```bash
kubectl apply -f networkpolicy.yaml
```

### 1.6 Horizontal Pod Autoscaling

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: lsp-max
  namespace: lsp-max
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: lsp-max
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
        - type: Percent
          value: 50
          periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 0
      policies:
        - type: Percent
          value: 100
          periodSeconds: 30
```

Apply:

```bash
kubectl apply -f hpa.yaml
```

---

## 2. Docker Compose Deployment

### 2.1 Single Container with Dependencies

```yaml
version: '3.8'

services:
  lsp-max:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: lsp-max
    ports:
      - "8080:8080"
      - "9091:9091"
    environment:
      LSP_MAX_LOG_LEVEL: info
      OTEL_EXPORTER_OTLP_ENDPOINT: http://otel-collector:4317
      GIT_TOKEN: ${GIT_TOKEN}
    volumes:
      - ./config.yaml:/etc/lsp-max/config.yaml:ro
      - lsp-sessions:/tmp/lsp-max-sessions
      - lsp-logs:/var/log/lsp-max
      - /path/to/workspace:/workspace:ro
    depends_on:
      otel-collector:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/healthz"]
      interval: 10s
      timeout: 5s
      retries: 3
      start_period: 30s
    networks:
      - lsp-max-network

  otel-collector:
    image: otel/opentelemetry-collector-contrib:latest
    container_name: otel-collector
    command:
      - "--config=/etc/otel-collector-config.yml"
    volumes:
      - ./otel-config.yml:/etc/otel-collector-config.yml:ro
    ports:
      - "4317:4317"  # gRPC
      - "4318:4318"  # HTTP
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:13133"]
      interval: 10s
      timeout: 5s
      retries: 3
    networks:
      - lsp-max-network

  prometheus:
    image: prom/prometheus:latest
    container_name: prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    networks:
      - lsp-max-network

  grafana:
    image: grafana/grafana:latest
    container_name: grafana
    ports:
      - "3000:3000"
    environment:
      GF_SECURITY_ADMIN_PASSWORD: ${GRAFANA_PASSWORD:-admin}
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana-datasources.yml:/etc/grafana/provisioning/datasources/datasources.yml:ro
    depends_on:
      - prometheus
    networks:
      - lsp-max-network

volumes:
  lsp-sessions:
  lsp-logs:
  prometheus-data:
  grafana-data:

networks:
  lsp-max-network:
    driver: bridge
```

**Environment file** (`.env`):

```bash
GIT_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
GRAFANA_PASSWORD=secure_password_here
```

**Start:**

```bash
docker-compose up -d
docker-compose logs -f lsp-max
```

### 2.2 Multi-Region Deployment

```yaml
version: '3.8'

services:
  # Primary region (us-west-2)
  lsp-max-primary:
    build: .
    container_name: lsp-max-primary
    ports:
      - "8080:8080"
    environment:
      LSP_MAX_REGION: us-west-2
      LSP_MAX_INSTANCE_ID: primary-001
    volumes:
      - lsp-primary-logs:/var/log/lsp-max
    networks:
      - lsp-max-mesh

  # Secondary region (us-east-1)
  lsp-max-secondary:
    build: .
    container_name: lsp-max-secondary
    ports:
      - "8081:8080"
    environment:
      LSP_MAX_REGION: us-east-1
      LSP_MAX_INSTANCE_ID: secondary-001
    volumes:
      - lsp-secondary-logs:/var/log/lsp-max
    networks:
      - lsp-max-mesh

  # Load balancer
  nginx:
    image: nginx:latest
    container_name: lsp-max-lb
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
    depends_on:
      - lsp-max-primary
      - lsp-max-secondary
    networks:
      - lsp-max-mesh

volumes:
  lsp-primary-logs:
  lsp-secondary-logs:

networks:
  lsp-max-mesh:
    driver: bridge
```

---

## 3. AWS Deployment

### 3.1 ECS on Fargate

```json
{
  "family": "lsp-max",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "1024",
  "memory": "2048",
  "containerDefinitions": [
    {
      "name": "lsp-max",
      "image": "123456789.dkr.ecr.us-west-2.amazonaws.com/lsp-max:26.7.1",
      "portMappings": [
        {
          "containerPort": 8080,
          "hostPort": 8080,
          "protocol": "tcp"
        },
        {
          "containerPort": 9091,
          "hostPort": 9091,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {
          "name": "LSP_MAX_LOG_LEVEL",
          "value": "info"
        },
        {
          "name": "OTEL_EXPORTER_OTLP_ENDPOINT",
          "value": "http://otel-collector.internal:4317"
        }
      ],
      "secrets": [
        {
          "name": "GIT_TOKEN",
          "valueFrom": "arn:aws:secretsmanager:us-west-2:123456789:secret:lsp-max/git-token:token::"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/lsp-max",
          "awslogs-region": "us-west-2",
          "awslogs-stream-prefix": "ecs"
        }
      },
      "healthCheck": {
        "command": ["CMD-SHELL", "curl -f http://localhost:8080/healthz || exit 1"],
        "interval": 10,
        "timeout": 5,
        "retries": 3,
        "startPeriod": 30
      }
    }
  ],
  "taskRoleArn": "arn:aws:iam::123456789:role/ecsTaskRole",
  "executionRoleArn": "arn:aws:iam::123456789:role/ecsTaskExecutionRole"
}
```

**CloudFormation template** (`lsp-max-stack.yaml`):

```yaml
AWSTemplateFormatVersion: '2010-09-09'
Description: 'Deploy lsp-max on ECS Fargate'

Parameters:
  VpcId:
    Type: AWS::EC2::VPC::Id
  SubnetIds:
    Type: List<AWS::EC2::Subnet::Id>
  SecurityGroupIds:
    Type: List<AWS::EC2::SecurityGroup::Id>

Resources:
  LspMaxCluster:
    Type: AWS::ECS::Cluster
    Properties:
      ClusterName: lsp-max-cluster

  LspMaxService:
    Type: AWS::ECS::Service
    Properties:
      Cluster: !Ref LspMaxCluster
      TaskDefinition: lsp-max:1
      DesiredCount: 3
      LaunchType: FARGATE
      NetworkConfiguration:
        AwsvpcConfiguration:
          AssignPublicIp: ENABLED
          Subnets: !Ref SubnetIds
          SecurityGroups: !Ref SecurityGroupIds
      LoadBalancers:
        - ContainerName: lsp-max
          ContainerPort: 8080
          TargetGroupArn: !Ref TargetGroup

  TargetGroup:
    Type: AWS::ElasticLoadBalancingV2::TargetGroup
    Properties:
      Port: 8080
      Protocol: HTTP
      TargetType: ip
      VpcId: !Ref VpcId
      HealthCheckPath: /healthz
      HealthCheckProtocol: HTTP

  LoadBalancer:
    Type: AWS::ElasticLoadBalancingV2::LoadBalancer
    Properties:
      Subnets: !Ref SubnetIds
      SecurityGroups: !Ref SecurityGroupIds

Outputs:
  LoadBalancerDNS:
    Value: !GetAtt LoadBalancer.DNSName
```

**Deploy:**

```bash
aws cloudformation create-stack \
  --stack-name lsp-max-stack \
  --template-body file://lsp-max-stack.yaml \
  --parameters ParameterKey=VpcId,ParameterValue=vpc-xxx \
                ParameterKey=SubnetIds,ParameterValue=subnet-xxx,subnet-yyy

aws ecs create-service \
  --cluster lsp-max-cluster \
  --service-name lsp-max \
  --task-definition lsp-max:1 \
  --desired-count 3 \
  --launch-type FARGATE
```

### 3.2 EC2 Auto Scaling Group

```yaml
AWSTemplateFormatVersion: '2010-09-09'

Resources:
  LaunchTemplate:
    Type: AWS::EC2::LaunchTemplate
    Properties:
      LaunchTemplateData:
        ImageId: ami-0c55b159cbfafe1f0  # Ubuntu 22.04
        InstanceType: t3.large
        KeyName: my-key-pair
        SecurityGroupIds:
          - sg-0123456789abcdef0
        UserData:
          Fn::Base64: |
            #!/bin/bash
            set -e
            
            # Install Docker
            apt-get update
            apt-get install -y docker.io docker-compose
            systemctl start docker
            
            # Clone lsp-max repository
            cd /opt
            git clone https://github.com/seanchatmangpt/lsp-max.git
            cd lsp-max
            
            # Start container
            docker-compose up -d
            
            # CloudWatch agent
            wget https://s3.amazonaws.com/amazoncloudwatch-agent/ubuntu/amd64/latest/amazon-cloudwatch-agent.deb
            dpkg -i amazon-cloudwatch-agent.deb
            /opt/aws/amazon-cloudwatch-agent/bin/amazon-cloudwatch-agent-ctl \
              -a fetch-config \
              -m ec2 \
              -s \
              -c ssm:lsp-max-cloudwatch-config

  AutoScalingGroup:
    Type: AWS::AutoScaling::AutoScalingGroup
    Properties:
      LaunchTemplate:
        LaunchTemplateId: !Ref LaunchTemplate
        Version: !GetAtt LaunchTemplate.LatestVersionNumber
      MinSize: 2
      MaxSize: 10
      DesiredCapacity: 3
      VPCZoneIdentifier:
        - subnet-0123456789abcdef0
        - subnet-fedcba9876543210
      TargetGroupARNs:
        - !Ref TargetGroup

  ScalingPolicy:
    Type: AWS::AutoScaling::ScalingPolicy
    Properties:
      AdjustmentType: ChangeInCapacity
      AutoScalingGroupName: !Ref AutoScalingGroup
      PolicyType: TargetTrackingScaling
      TargetTrackingConfiguration:
        PredefinedMetricSpecification:
          PredefinedMetricType: ASGAverageCPUUtilization
        TargetValue: 70.0
```

---

## 4. Google Cloud Platform (GCP)

### 4.1 Cloud Run Deployment

```bash
# Build and push to Container Registry
gcloud builds submit --tag gcr.io/$PROJECT_ID/lsp-max:26.7.1

# Deploy to Cloud Run
gcloud run deploy lsp-max \
  --image gcr.io/$PROJECT_ID/lsp-max:26.7.1 \
  --platform managed \
  --region us-central1 \
  --memory 2Gi \
  --cpu 2 \
  --timeout 3600 \
  --set-env-vars LSP_MAX_LOG_LEVEL=info,OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317 \
  --set-secrets GIT_TOKEN=lsp-max-git-token:latest \
  --no-allow-unauthenticated \
  --min-instances 1 \
  --max-instances 10
```

### 4.2 GKE (Google Kubernetes Engine)

```bash
# Create cluster
gcloud container clusters create lsp-max-cluster \
  --zone us-central1-a \
  --num-nodes 3 \
  --machine-type n2-standard-4 \
  --enable-autoscaling \
  --min-nodes 2 \
  --max-nodes 10 \
  --enable-ip-alias \
  --enable-network-policy

# Deploy application (use Kubernetes manifests from section 1)
kubectl apply -f namespace.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service-ingress.yaml
```

### 4.3 Cloud Firestore for State (Optional)

For stateful deployments, store session metadata in Firestore:

```rust
use google_cloud_firestore::Client;

async fn store_session_metadata(
    client: &Client,
    session_id: &str,
    metadata: SessionMetadata,
) -> Result<()> {
    client
        .collection("sessions")
        .document(session_id)
        .set(serde_json::to_value(&metadata)?)
        .await?;
    Ok(())
}
```

---

## 5. Custom Agent Integration

### 5.1 Agent Client Setup

```rust
// agent/src/lsp_client.rs
use lsp_max_client::Client;
use lsp_max_client::transport::stdio::PipeTransport;
use tokio::process::Command;

async fn spawn_lsp_server() -> Result<Client> {
    let mut cmd = Command::new("lsp-max-server");
    
    let process = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    
    let transport = PipeTransport::new(
        process.stdout.unwrap(),
        process.stdin.unwrap(),
    );
    
    let client = Client::new(transport);
    
    // Initialize
    let init_result = client
        .send_request::<InitializeRequest>(InitializeParams {
            ..Default::default()
        })
        .await?;
    
    client.notify::<InitializedNotification>(InitializedParams {}).await?;
    
    Ok(client)
}
```

### 5.2 Agent Query Pattern

```rust
// agent/src/analysis.rs
async fn analyze_document(
    client: &Client,
    uri: Url,
    content: String,
) -> Result<AnalysisResult> {
    // Open document
    client.notify::<DidOpenTextDocument>(
        DidOpenTextDocumentParams {
            textDocument: TextDocumentItem {
                uri: uri.clone(),
                languageId: "rust".to_string(),
                version: 1,
                text: content,
            },
        }
    ).await?;
    
    // Query hover at multiple positions
    let hovers = futures::future::join_all(
        (0..100).map(|pos| {
            client.send_request::<HoverRequest>(HoverParams {
                textDocument: TextDocumentPositionParams {
                    textDocument: TextDocumentIdentifier {
                        uri: uri.clone(),
                    },
                    position: Position {
                        line: 0,
                        character: pos,
                    },
                },
                workDoneProgressParams: Default::default(),
            })
        })
    ).await;
    
    // Check gate status
    let gate_status = client.send_request::<MaxGateStatusRequest>(json!({})).await?;
    
    Ok(AnalysisResult {
        hovers,
        gate_open: gate_status.state == "OPEN",
    })
}
```

### 5.3 Docker Network for Agent System

```yaml
version: '3.8'

services:
  agent-orchestrator:
    build:
      context: ./agent
      dockerfile: Dockerfile
    container_name: agent-orchestrator
    environment:
      LSP_MAX_SERVER: http://lsp-max:8080
      AGENT_ID: orchestrator-001
    networks:
      - agent-mesh
    depends_on:
      - lsp-max

  lsp-max:
    build:
      context: ./lsp-max
    container_name: lsp-max
    environment:
      LSP_MAX_LOG_LEVEL: debug
    networks:
      - agent-mesh
    expose:
      - "8080"

  # Worker agents
  agent-worker-1:
    build:
      context: ./agent
      dockerfile: Dockerfile
    container_name: agent-worker-1
    environment:
      LSP_MAX_SERVER: http://lsp-max:8080
      AGENT_ID: worker-001
      WORKER_POOL_SIZE: 10
    networks:
      - agent-mesh
    depends_on:
      - lsp-max

  agent-worker-2:
    build:
      context: ./agent
      dockerfile: Dockerfile
    container_name: agent-worker-2
    environment:
      LSP_MAX_SERVER: http://lsp-max:8080
      AGENT_ID: worker-002
      WORKER_POOL_SIZE: 10
    networks:
      - agent-mesh
    depends_on:
      - lsp-max

networks:
  agent-mesh:
    driver: bridge
```

---

## 6. GitHub Actions Workflow

### 6.1 Matrix Testing

```yaml
name: lsp-max-matrix-test

on:
  push:
    branches: [main, release/*]
  pull_request:
    branches: [main, release/*]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, nightly]
        include:
          - os: ubuntu-latest
            rust: stable
            coverage: true
    
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Checkout dependencies
        run: |
          cd ..
          for repo in lsp-types-max wasm4pm-compat wasm4pm; do
            [ -d "$repo" ] || git clone "https://github.com/seanchatmangpt/$repo.git"
          done
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
          components: rustfmt, clippy
      
      - name: Cache Cargo
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/bin
            ~/.cargo/registry/index
            ~/.cargo/registry/cache
            ~/.cargo/git/db
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: ${{ runner.os }}-cargo-
      
      - name: Run tests
        run: cargo test --workspace --locked
      
      - name: Run clippy
        if: matrix.rust == 'stable'
        run: cargo clippy --workspace --all-targets -- -D warnings
      
      - name: Generate coverage
        if: matrix.coverage
        run: |
          cargo install tarpaulin
          cargo tarpaulin --out Xml --output-dir coverage
      
      - name: Upload coverage
        if: matrix.coverage
        uses: codecov/codecov-action@v3
        with:
          files: coverage/cobertura.xml
```

### 6.2 Release Workflow

```yaml
name: release

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Checkout dependencies
        run: |
          cd ..
          for repo in lsp-types-max wasm4pm-compat wasm4pm; do
            [ -d "$repo" ] || git clone "https://github.com/seanchatmangpt/$repo.git"
          done
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Verify version
        run: |
          VERSION=$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name=="lsp-max") | .version')
          TAG=${GITHUB_REF#refs/tags/}
          if [ "$VERSION" != "$TAG" ]; then
            echo "Version mismatch: $VERSION vs $TAG"
            exit 1
          fi
      
      - name: Test
        run: cargo test --release --locked
      
      - name: Publish to crates.io
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_TOKEN }}
        run: cargo publish
      
      - name: Create Release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref }}
          body: |
            ## Changes
            - See CHANGELOG.md for details
          draft: false
          prerelease: false
      
      - name: Build and push Docker image
        run: |
          docker build -t gcr.io/my-project/lsp-max:${GITHUB_REF#refs/tags/} .
          docker push gcr.io/my-project/lsp-max:${GITHUB_REF#refs/tags/}
```

---

## 7. Monitoring & Logging Setup

### 7.1 Prometheus Configuration

`prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'lsp-max'
    static_configs:
      - targets: ['localhost:9091']
    scrape_interval: 10s
    scrape_timeout: 5s
    metrics_path: /metrics
```

### 7.2 Grafana Dashboard

```json
{
  "dashboard": {
    "title": "lsp-max",
    "panels": [
      {
        "title": "Request Latency (P95)",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, lsp_max_request_duration_ms)"
          }
        ]
      },
      {
        "title": "Active Sessions",
        "targets": [
          {
            "expr": "lsp_max_session_active"
          }
        ]
      },
      {
        "title": "Gate State",
        "targets": [
          {
            "expr": "lsp_max_gate_state"
          }
        ]
      }
    ]
  }
}
```

---

## Summary

This guide covers deployment to:
- **Kubernetes**: Production-grade multi-replica deployments with HPA and network policies
- **Docker Compose**: Local development and single-machine deployments
- **AWS**: ECS Fargate, EC2 Auto Scaling
- **GCP**: Cloud Run and GKE
- **Custom agents**: Integration with agent orchestration systems
- **GitHub Actions**: CI/CD automation and releases

For additional reference, see:
- `/home/user/lsp-max/docs/REMOTE_EXECUTION.md` — Container isolation and resource limits
- `/home/user/lsp-max/CLAUDE.md` — Claude Code integration
- `/home/user/lsp-max/.github/workflows/ci.yml` — Existing CI configuration
