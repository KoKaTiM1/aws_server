# Elastic IPs for NAT Gateways
resource "aws_eip" "nat" {
  count  = var.nat_gateway_count
  domain = "vpc"

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-nat-eip-${count.index + 1}"
  })

  depends_on = [var.vpc_id]
}

# NAT Gateways
resource "aws_nat_gateway" "main" {
  count         = var.nat_gateway_count
  allocation_id = aws_eip.nat[count.index].id
  subnet_id     = var.public_subnet_ids[count.index]

  tags = merge(var.tags, {
    Name = "eyedar-${var.env_name}-nat-${count.index + 1}"
  })
}

# Add NAT route to private route tables
# If we have 1 NAT, all private subnets use it
# If we have 2+ NATs, each private subnet uses the NAT in its AZ
resource "aws_route" "private_nat" {
  count                  = length(var.private_route_table_ids)
  route_table_id         = var.private_route_table_ids[count.index]
  destination_cidr_block = "0.0.0.0/0"
  nat_gateway_id         = aws_nat_gateway.main[min(count.index, var.nat_gateway_count - 1)].id
}
