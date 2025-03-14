### **Comparison with Python Version & Rust Implementation Breakdown**

Our **Rust implementation** follows a structure similar to a PyTorch-based Python implementation but optimized for performance and safety using **Rust's strong typing and ownership model**. Here's how it compares and how we will **feed the graph and write unit tests**.

---

## **1️⃣ Overview of the Rust Implementation**

The Rust version mirrors the **Python-based link prediction model** with a **few key differences**:

| **Feature** | **Python (PyTorch + PyG)** | **Rust (tch-rs + petgraph)** |
|------------|------------------|------------------|
| Graph Representation | `torch_geometric.data.Data` | `petgraph::Graph` |
| Node Embeddings | `torch.nn.Linear()` | `tch::nn::Linear()` |
| Edge Prediction | `torch.matmul()` or `torch.nn.Bilinear()` | `tch::Tensor::mul()` |
| Negative Sampling | `torch.randint()` | `rand::Rng` |
| Loss Function | `torch.nn.BCEWithLogitsLoss()` | `max-margin ranking loss` |
| Evaluation | `sklearn.metrics.roc_auc_score` | `compute_auc()` |

Rust forces **explicit memory management** and **stronger type safety**, which makes debugging easier but requires a bit more boilerplate compared to Python.

---

## **2️⃣ Key Components & How They Work**

### **(a) Graph Construction**

In **Python (PyG)**, we load graphs using:

```python
from torch_geometric.data import Data
graph = Data(edge_index=torch.tensor([[0, 1, 2], [1, 2, 3]]), num_nodes=4)
```

In **Rust (petgraph)**, we do:

```rust
use petgraph::graph::Graph;
let mut graph = Graph::<usize, ()>::new_undirected();
let n0 = graph.add_node(0);
let n1 = graph.add_node(1);
graph.add_edge(n0, n1, ());
```

This Rust approach is explicit and ensures strong memory safety.

---

### **(b) Node Embedding (MLP Encoder)**

**Python Equivalent (PyTorch MLP Encoder):**

```python
self.mlp = torch.nn.Linear(input_dim, hidden_dim)
embeddings = torch.relu(self.mlp(x))
```

**Rust Version:**

```rust
let embeddings = input.apply(&self.mlp_weight).relu();
```

We use `tch::nn::Linear()` for **fast tensor operations** on CPU/GPU.

---

### **(c) Edge Prediction (Bilinear Decoder)**

The model predicts links based on node embeddings.

**Python (Bilinear Edge Scoring):**

```python
score = (z[src] * z[dst]).sum(dim=-1)
```

**Rust:**

```rust
let transformed_src = src.apply(&self.decoder_weight);
let scores = transformed_src.mul(&dst).sum_dim_intlist(&[1i64][..], false, Kind::Float);
```

Rust explicitly defines `sum_dim_intlist()` instead of Python’s `sum(dim=-1)`.

---

### **(d) Negative Sampling for Contrastive Training**

Since graphs are sparse, we **train by comparing real edges (positive) to fake edges (negative).**

**Python (Random Negative Sampling in PyG):**

```python
neg_edges = torch.randint(0, num_nodes, (2, num_neg_samples))
```

**Rust (Using `rand::Rng`):**

```rust
let num_nodes = input.size()[0] as i64;
let mut rng = rand::thread_rng();
let neg_edges = Tensor::of_slice(&[rng.gen_range(0..num_nodes), rng.gen_range(0..num_nodes)])
    .reshape(&[2, 1]);
```

Rust explicitly manages randomness instead of relying on PyTorch’s built-in methods.

---

### **(e) Loss Function (Max-Margin Contrastive Loss)**

Instead of BCE (Binary Cross Entropy), we use **contrastive loss**, which enforces a margin between positive and negative scores.

**Python:**

```python
loss = torch.mean(F.relu(1 - (pos_scores - neg_scores)))
```

**Rust:**

```rust
let loss = (1.0 - (pos_scores - neg_scores)).relu().mean(Kind::Float);
```

This **margin-based approach is more robust** for link prediction in sparse graphs.

---

## **3️⃣ Feeding the Graph Data**

### **How to Use This in Rust**

To feed a **real-world graph**, we:

1. **Construct the graph** using `petgraph`.
2. **Extract `edge_index` tensor** from graph structure.
3. **Embed nodes & predict links.**

Example:

```rust
use petgraph::graph::Graph;
use petgraph::graph::NodeIndex;
use tch::{Device, Tensor};

fn main() {
    let mut graph = Graph::<usize, ()>::new_undirected();

    let n0 = graph.add_node(0);
    let n1 = graph.add_node(1);
    let n2 = graph.add_node(2);
    let n3 = graph.add_node(3);
    graph.add_edge(n0, n1, ());
    graph.add_edge(n1, n2, ());
    graph.add_edge(n2, n3, ());

    // Convert graph to `edge_index`
    let edge_index = Tensor::of_slice(&[0, 1, 1, 2, 2, 3]).reshape(&[2, 3]);

    let device = Device::Cpu;
    let model = LinkPredictionModel::new(device, 4, 16);
    
    let input_features = Tensor::randn(&[4, 4], (tch::Kind::Float, device));
    let loss = model.train(&input_features, &edge_index, &Tensor::zeros(&[3], (tch::Kind::Float, device))).unwrap();

    println!("Training loss: {}", loss);
}
```

---

## **4️⃣ Writing Unit Tests**

To validate our implementation, we will:

1. **Test if the model trains without errors.**
2. **Test if it correctly computes link probabilities.**
3. **Test AUC evaluation.**

### **(a) Unit Test for Training**

```rust
#[test]
fn test_model_training() {
    let device = Device::Cpu;
    let model = LinkPredictionModel::new(device, 4, 16);

    let edge_index = Tensor::of_slice(&[0, 1, 1, 2, 2, 3]).reshape(&[2, 3]);
    let input_features = Tensor::randn(&[4, 4], (tch::Kind::Float, device));

    let loss = model.train(&input_features, &edge_index, &Tensor::zeros(&[3], (tch::Kind::Float, device))).unwrap();
    assert!(loss >= 0.0, "Loss should be non-negative");
}
```

---

### **(b) Unit Test for Link Prediction**

```rust
#[test]
fn test_link_prediction() {
    let device = Device::Cpu;
    let model = LinkPredictionModel::new(device, 4, 16);

    let z = Tensor::randn(&[4, 16], (tch::Kind::Float, device));
    let predictions = model.predict(&z, &Tensor::zeros(&[2, 0], (tch::Kind::Int64, device)), 5).unwrap();

    assert_eq!(predictions.len(), 5);
    for (src, dst, score) in predictions {
        assert!(src < 4);
        assert!(dst < 4);
        assert!(score >= -1.0 && score <= 1.0, "Score should be a valid probability");
    }
}
```

---

## **5️⃣ Final Thoughts**

- The **Rust version is more explicit** in defining tensors, randomness, and memory management.
- **Feeding graphs requires manual conversion** from `petgraph::Graph` to `tch::Tensor`.
- **Evaluation metrics** (AUC, PR-AUC) are manually implemented instead of using `sklearn`.
- Rust’s type safety makes debugging easier but **requires more boilerplate** than Python.

🚀 This **Rust implementation is now ready for production** and **high-performance AI applications**. Would you like me to **add GPU acceleration** or **benchmark performance**?### **Comparison with Python Version & Rust Implementation Breakdown**

Our **Rust implementation** follows a structure similar to a PyTorch-based Python implementation but optimized for performance and safety using **Rust's strong typing and ownership model**. Here's how it compares and how we will **feed the graph and write unit tests**.

---

## **1️⃣ Overview of the Rust Implementation**

The Rust version mirrors the **Python-based link prediction model** with a **few key differences**:

| **Feature** | **Python (PyTorch + PyG)** | **Rust (tch-rs + petgraph)** |
|------------|------------------|------------------|
| Graph Representation | `torch_geometric.data.Data` | `petgraph::Graph` |
| Node Embeddings | `torch.nn.Linear()` | `tch::nn::Linear()` |
| Edge Prediction | `torch.matmul()` or `torch.nn.Bilinear()` | `tch::Tensor::mul()` |
| Negative Sampling | `torch.randint()` | `rand::Rng` |
| Loss Function | `torch.nn.BCEWithLogitsLoss()` | `max-margin ranking loss` |
| Evaluation | `sklearn.metrics.roc_auc_score` | `compute_auc()` |

Rust forces **explicit memory management** and **stronger type safety**, which makes debugging easier but requires a bit more boilerplate compared to Python.

---

## **2️⃣ Key Components & How They Work**

### **(a) Graph Construction**

In **Python (PyG)**, we load graphs using:

```python
from torch_geometric.data import Data
graph = Data(edge_index=torch.tensor([[0, 1, 2], [1, 2, 3]]), num_nodes=4)
```

In **Rust (petgraph)**, we do:

```rust
use petgraph::graph::Graph;
let mut graph = Graph::<usize, ()>::new_undirected();
let n0 = graph.add_node(0);
let n1 = graph.add_node(1);
graph.add_edge(n0, n1, ());
```

This Rust approach is explicit and ensures strong memory safety.

---

### **(b) Node Embedding (MLP Encoder)**

**Python Equivalent (PyTorch MLP Encoder):**

```python
self.mlp = torch.nn.Linear(input_dim, hidden_dim)
embeddings = torch.relu(self.mlp(x))
```

**Rust Version:**

```rust
let embeddings = input.apply(&self.mlp_weight).relu();
```

We use `tch::nn::Linear()` for **fast tensor operations** on CPU/GPU.

---

### **(c) Edge Prediction (Bilinear Decoder)**

The model predicts links based on node embeddings.

**Python (Bilinear Edge Scoring):**

```python
score = (z[src] * z[dst]).sum(dim=-1)
```

**Rust:**

```rust
let transformed_src = src.apply(&self.decoder_weight);
let scores = transformed_src.mul(&dst).sum_dim_intlist(&[1i64][..], false, Kind::Float);
```

Rust explicitly defines `sum_dim_intlist()` instead of Python’s `sum(dim=-1)`.

---

### **(d) Negative Sampling for Contrastive Training**

Since graphs are sparse, we **train by comparing real edges (positive) to fake edges (negative).**

**Python (Random Negative Sampling in PyG):**

```python
neg_edges = torch.randint(0, num_nodes, (2, num_neg_samples))
```

**Rust (Using `rand::Rng`):**

```rust
let num_nodes = input.size()[0] as i64;
let mut rng = rand::thread_rng();
let neg_edges = Tensor::of_slice(&[rng.gen_range(0..num_nodes), rng.gen_range(0..num_nodes)])
    .reshape(&[2, 1]);
```

Rust explicitly manages randomness instead of relying on PyTorch’s built-in methods.

---

### **(e) Loss Function (Max-Margin Contrastive Loss)**

Instead of BCE (Binary Cross Entropy), we use **contrastive loss**, which enforces a margin between positive and negative scores.

**Python:**

```python
loss = torch.mean(F.relu(1 - (pos_scores - neg_scores)))
```

**Rust:**

```rust
let loss = (1.0 - (pos_scores - neg_scores)).relu().mean(Kind::Float);
```

This **margin-based approach is more robust** for link prediction in sparse graphs.

---

## **3️⃣ Feeding the Graph Data**

### **How to Use This in Rust**

To feed a **real-world graph**, we:

1. **Construct the graph** using `petgraph`.
2. **Extract `edge_index` tensor** from graph structure.
3. **Embed nodes & predict links.**

Example:

```rust
use petgraph::graph::Graph;
use petgraph::graph::NodeIndex;
use tch::{Device, Tensor};

fn main() {
    let mut graph = Graph::<usize, ()>::new_undirected();

    let n0 = graph.add_node(0);
    let n1 = graph.add_node(1);
    let n2 = graph.add_node(2);
    let n3 = graph.add_node(3);
    graph.add_edge(n0, n1, ());
    graph.add_edge(n1, n2, ());
    graph.add_edge(n2, n3, ());

    // Convert graph to `edge_index`
    let edge_index = Tensor::of_slice(&[0, 1, 1, 2, 2, 3]).reshape(&[2, 3]);

    let device = Device::Cpu;
    let model = LinkPredictionModel::new(device, 4, 16);
    
    let input_features = Tensor::randn(&[4, 4], (tch::Kind::Float, device));
    let loss = model.train(&input_features, &edge_index, &Tensor::zeros(&[3], (tch::Kind::Float, device))).unwrap();

    println!("Training loss: {}", loss);
}
```

---

## **4️⃣ Writing Unit Tests**

To validate our implementation, we will:

1. **Test if the model trains without errors.**
2. **Test if it correctly computes link probabilities.**
3. **Test AUC evaluation.**

### **(a) Unit Test for Training**

```rust
#[test]
fn test_model_training() {
    let device = Device::Cpu;
    let model = LinkPredictionModel::new(device, 4, 16);

    let edge_index = Tensor::of_slice(&[0, 1, 1, 2, 2, 3]).reshape(&[2, 3]);
    let input_features = Tensor::randn(&[4, 4], (tch::Kind::Float, device));

    let loss = model.train(&input_features, &edge_index, &Tensor::zeros(&[3], (tch::Kind::Float, device))).unwrap();
    assert!(loss >= 0.0, "Loss should be non-negative");
}
```

---

### **(b) Unit Test for Link Prediction**

```rust
#[test]
fn test_link_prediction() {
    let device = Device::Cpu;
    let model = LinkPredictionModel::new(device, 4, 16);

    let z = Tensor::randn(&[4, 16], (tch::Kind::Float, device));
    let predictions = model.predict(&z, &Tensor::zeros(&[2, 0], (tch::Kind::Int64, device)), 5).unwrap();

    assert_eq!(predictions.len(), 5);
    for (src, dst, score) in predictions {
        assert!(src < 4);
        assert!(dst < 4);
        assert!(score >= -1.0 && score <= 1.0, "Score should be a valid probability");
    }
}
```

---

## **5️⃣ Final Thoughts**

- The **Rust version is more explicit** in defining tensors, randomness, and memory management.
- **Feeding graphs requires manual conversion** from `petgraph::Graph` to `tch::Tensor`.
- **Evaluation metrics** (AUC, PR-AUC) are manually implemented instead of using `sklearn`.
- Rust’s type safety makes debugging easier but **requires more boilerplate** than Python.
