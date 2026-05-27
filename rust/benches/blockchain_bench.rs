use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use llm_mina_chain::{Blockchain, Transaction, State, Block, KeyPair};

fn bench_transaction_creation(c: &mut Criterion) {
    c.bench_function("transaction_creation", |b| {
        b.iter(|| {
            Transaction::new(
                black_box("alice".to_string()),
                black_box("bob".to_string()),
                black_box(100),
                black_box(0),
                black_box(Some(21000)),
                black_box(Some(1)),
            )
        });
    });
}

fn bench_transaction_hash(c: &mut Criterion) {
    let tx = Transaction::new(
        "alice".to_string(),
        "bob".to_string(),
        100,
        0,
        Some(21000),
        Some(1),
    );
    
    c.bench_function("transaction_hash", |b| {
        b.iter(|| black_box(tx.hash()));
    });
}

fn bench_state_application(c: &mut Criterion) {
    let mut state = State::new();
    state.set_balance("alice".to_string(), 1000);
    state.set_balance("bob".to_string(), 500);
    
    let tx = Transaction::new(
        "alice".to_string(),
        "bob".to_string(),
        100,
        0,
        None,
        None,
    );
    
    c.bench_function("state_application", |b| {
        b.iter(|| {
            let mut s = State {
                balances: state.balances.clone(),
                nonces: state.nonces.clone(),
                contracts: state.contracts.clone(),
            };
            s.apply_transaction(black_box(&tx));
        });
    });
}

fn bench_block_creation(c: &mut Criterion) {
    let mut blockchain = Blockchain::new();
    
    c.bench_function("block_creation", |b| {
        b.iter(|| {
            let tx = Transaction::new(
                "alice".to_string(),
                "bob".to_string(),
                100,
                black_box(0),
                None,
                None,
            );
            blockchain.create_block(vec![tx]);
        });
    });
}

fn bench_block_hash(c: &mut Criterion) {
    let block = Block::new(
        1,
        vec![],
        "prev_hash".to_string(),
        "state_hash".to_string(),
    );
    
    c.bench_function("block_hash", |b| {
        b.iter(|| black_box(block.compute_hash()));
    });
}

fn bench_keypair_generation(c: &mut Criterion) {
    c.bench_function("keypair_generation", |b| {
        b.iter(|| KeyPair::generate());
    });
}

fn bench_transaction_signing(c: &mut Criterion) {
    let keypair = KeyPair::generate();
    let mut tx = Transaction::new(
        "alice".to_string(),
        "bob".to_string(),
        100,
        0,
        None,
        None,
    );
    
    c.bench_function("transaction_signing", |b| {
        b.iter(|| {
            let mut t = tx.clone();
            t.sign(black_box(&keypair));
        });
    });
}

fn bench_transaction_verification(c: &mut Criterion) {
    let keypair = KeyPair::generate();
    let mut tx = Transaction::new(
        "alice".to_string(),
        "bob".to_string(),
        100,
        0,
        None,
        None,
    );
    tx.sign(&keypair);
    
    c.bench_function("transaction_verification", |b| {
        b.iter(|| {
            tx.verify_signature(black_box(&keypair.public_key));
        });
    });
}

fn bench_block_with_many_transactions(c: &mut Criterion) {
    let mut group = c.benchmark_group("block_with_transactions");
    
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut blockchain = Blockchain::new();
            let transactions: Vec<Transaction> = (0..size)
                .map(|i| {
                    Transaction::new(
                        "alice".to_string(),
                        format!("user{}", i),
                        10,
                        i as u64,
                        None,
                        None,
                    )
                })
                .collect();
            
            b.iter(|| {
                let mut bc = Blockchain::new();
                bc.create_block(black_box(transactions.clone()));
            });
        });
    }
    
    group.finish();
}

fn bench_state_hash(c: &mut Criterion) {
    let mut state = State::new();
    state.set_balance("alice".to_string(), 1000);
    state.set_balance("bob".to_string(), 500);
    
    c.bench_function("state_hash", |b| {
        b.iter(|| black_box(state.hash()));
    });
}

criterion_group!(
    benches,
    bench_transaction_creation,
    bench_transaction_hash,
    bench_state_application,
    bench_block_creation,
    bench_block_hash,
    bench_keypair_generation,
    bench_transaction_signing,
    bench_transaction_verification,
    bench_block_with_many_transactions,
    bench_state_hash
);
criterion_main!(benches);
