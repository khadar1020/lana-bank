resource "cala_big_query_integration" "bq" {
  count                        = local.setup_bq ? 1 : 0
  id                           = "00000000-0000-0000-0000-000000000001"
  name                         = "bq-integration"
  project_id                   = local.project_id
  dataset_id                   = local.dataset_id
  service_account_creds_base64 = var.sa_creds
}


resource "google_bigquery_table" "entities" {
  for_each            = toset(local.bq_entity_tables)
  project             = local.project_id
  dataset_id          = local.dataset_id
  table_id            = each.value
  deletion_protection = local.deletion_protection

  schema = <<EOF
[
  {
    "name": "id",
    "type": "STRING",
    "description": "The ID of the entity"
  },
  {
    "name": "event_type",
    "type": "STRING",
    "description": "The type of the event"
  },
  {
    "name": "event",
    "type": "JSON",
    "description": "The JSON of the event"
  },
  {
    "name": "sequence",
    "type": "INTEGER",
    "description": "The sequence number of the event"
  },
  {
    "name": "recorded_at",
    "type": "TIMESTAMP",
    "description": "When the event was recorded"
  }
]
EOF
}

resource "google_bigquery_table_iam_member" "entities_owner_sa" {
  for_each   = toset(local.bq_entity_tables)
  project    = local.project_id
  dataset_id = local.dataset_id
  table_id   = google_bigquery_table.entities[each.value].table_id
  role       = "roles/bigquery.dataOwner"
  member     = local.sa_member
}

resource "google_bigquery_table" "sumsub_applicants" {
  count = local.setup_bq ? 1 : 0

  project             = local.project_id
  dataset_id          = local.dataset_id
  table_id            = local.bq_applicant_table
  deletion_protection = local.deletion_protection

  schema = <<EOF
[
  {
    "name": "customer_id",
    "type": "STRING",
    "description": "The ID the customer entity"
  },
{
  "name": "content_type",
  "type": "STRING",
  "description": "The type of the content"
},
  {
    "name": "content",
    "type": "JSON",
    "description": "content JSON from Sum Sub"
  },
  {
    "name": "uploaded_at",
    "type": "TIMESTAMP",
    "description": "When the data was synced"
  }
]
EOF
}

resource "google_bigquery_table_iam_member" "applicants_owner_sa" {
  count      = local.setup_bq ? 1 : 0
  project    = local.project_id
  dataset_id = local.dataset_id
  table_id   = google_bigquery_table.sumsub_applicants[0].table_id
  role       = "roles/bigquery.dataOwner"
  member     = local.sa_member
}


resource "google_bigquery_table" "price_cents_btc" {
  count = local.setup_bq ? 1 : 0

  project             = local.project_id
  dataset_id          = local.dataset_id
  table_id            = local.bq_price_cents_btc
  deletion_protection = local.deletion_protection

  schema = <<EOF
[
  {
    "name": "usd_cents_per_btc",
    "type": "NUMERIC",
    "description": "The price of BTC in USD cents"
  },
  {
    "name": "uploaded_at",
    "type": "TIMESTAMP",
    "description": "When the price was recorded"
  }
]
EOF
}

resource "google_bigquery_table_iam_member" "price_cents_btc_owner_sa" {
  count      = local.setup_bq ? 1 : 0
  project    = local.project_id
  dataset_id = local.dataset_id
  table_id   = google_bigquery_table.price_cents_btc[0].table_id
  role       = "roles/bigquery.dataOwner"
  member     = local.sa_member
}
